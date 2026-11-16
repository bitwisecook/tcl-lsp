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

//! Original operand layouts observed in an original body's own frame.
//!
//! A layout observation precedes operand evaluation. It preserves candidate
//! lookup and logical grammar for diagnostics. Declared frames and original
//! entered frames retain their independent identities; this never publishes compiler
//! admission, successful dispatch, an entered body, or closed runtime effects.

use super::original_name_value::OriginalWordSourcePurpose;

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use super::{
    CommandAllocationSite, ModuleCommandBindings, OriginalCompilationLookupAdvice,
    SourceCommandBindings, SourceConditionalBodyEntry, SourceExecutionContext,
    SourceLookupSnapshot,
};

/// Original diagnostic frame ownership. A root script retains its actual
/// global frame; it never receives a synthetic procedure allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum OriginalDiagnosticFrameEntry {
    Body(Arc<SourceConditionalBodyEntry>),
    DeclaredReceiver(Arc<super::SourceDeclaredReceiverBodyEntry>),
    LogicalReceiver(Arc<OriginalLogicalReceiverSourceBody>),
    DeclaredProcedure(Arc<super::declaration_preview::DeclaredProcedureBody>),
    RootScript {
        source: super::ExecutedScriptSource,
        frame: crate::var_resolve::VariableExecutionFrame,
        namespace: super::SourceNamespaceKey,
    },
    ScriptActivation {
        source: super::ExecutedScriptSource,
        frame: crate::var_resolve::VariableExecutionFrame,
        namespace: super::SourceNamespaceKey,
        config: tcl_lexer::LexerConfig,
    },
    /// A conditional Logical source frame, with no entered Native activation.
    LogicalScriptActivation {
        source: super::ExecutedScriptSource,
        frame: crate::var_resolve::VariableExecutionFrame,
        namespace: super::SourceNamespaceKey,
        config: tcl_lexer::LexerConfig,
    },
}

/// Original procedure source scope for parameter reports. The actual row
/// retains either its allocated body or its uninstalled declaration; neither
/// variant supplies entered-frame or Native invocation authority.
pub(crate) struct OriginalBodyParameterSourceScope {
    entry: Arc<OriginalDiagnosticFrameEntry>,
    layout: OriginalCompilationLookupAdvice,
}
impl OriginalBodyParameterSourceScope {
    pub(crate) fn layout(&self) -> &OriginalCompilationLookupAdvice {
        &self.layout
    }
    pub(crate) fn matches_parameters(&self, parameters: &[&str]) -> bool {
        self.layout
            .closed_body_parameter_source_entry_lookup(&self.entry)
            && self
                .entry
                .original_formal_topology()
                .is_some_and(|formals| {
                    formals
                        .parameters()
                        .iter()
                        .map(|formal| formal.name.as_slice())
                        .eq(parameters.iter().map(|name| name.as_bytes()))
                })
    }
    pub(crate) fn original_words(
        &self,
        tokens: &crate::ir::CommandTokens,
        config: tcl_lexer::LexerConfig,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Vec<tcl_lexer::NativeWord>> {
        if let OriginalDiagnosticFrameEntry::Body(body) = self.entry.as_ref() {
            return self
                .layout
                .original_body_parameter_source_words(tokens, body, config, registry);
        }
        (self.layout.config == config).then_some(())?;
        self.layout
            .closed_body_parameter_source_entry_lookup(&self.entry)
            .then_some(())?;
        self.layout
            .original_declaration_metadata_context(registry, tokens)
            .ok()?;
        let original = self.layout.site.source.original_lexical_command(
            tokens.words(),
            self.layout.site.offset,
            config,
        )?;
        Some(original.native_words().to_vec())
    }
}

impl OriginalCompilationLookupAdvice {
    fn closed_body_parameter_source_entry_lookup(
        &self,
        entry: &OriginalDiagnosticFrameEntry,
    ) -> bool {
        let namespace = match entry {
            OriginalDiagnosticFrameEntry::Body(body) => body.namespace_context(),
            OriginalDiagnosticFrameEntry::DeclaredProcedure(body) => Some(&body.namespace),
            _ => return false,
        };
        self.closed_body_parameter_source_parts(
            entry.owns_source(&self.site.source, self.site.offset),
            entry.frame(),
            namespace,
        )
    }
}

/// A retained Logical member/body issuer with original source geometry only.
/// Its frame identity describes the written receiver body, without a Native
/// method allocation, formal ABI, receiver namespace or entered activation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OriginalLogicalReceiverSourceBody {
    body: Arc<crate::registry_invocation::OriginalSourceDefinitionMemberScriptBody>,
    input: crate::analyser::ResolvedAnalysisInput,
    source: super::ExecutedScriptSource,
    frame: crate::var_resolve::VariableExecutionFrame,
}

impl OriginalDiagnosticFrameEntry {
    pub(super) fn owns_original_context(
        &self,
        context: &crate::var_resolve::ResolveContext,
    ) -> bool {
        let namespace = match self {
            Self::Body(body) => body.namespace_context(),
            Self::DeclaredReceiver(_) => None,
            Self::LogicalReceiver(_) => return false,
            Self::DeclaredProcedure(body) => Some(&body.namespace),
            Self::RootScript { namespace, .. }
            | Self::ScriptActivation { namespace, .. }
            | Self::LogicalScriptActivation { namespace, .. } => Some(namespace),
        };
        SourceCommandBindings::context_owns_frame(context, self.frame(), namespace)
    }

    pub(super) fn source(&self) -> &super::ExecutedScriptSource {
        match self {
            Self::Body(body) => body.source(),
            Self::DeclaredReceiver(body) => body.source(),
            Self::LogicalReceiver(body) => &body.source,
            Self::DeclaredProcedure(body) => &body.source,
            Self::RootScript { source, .. }
            | Self::ScriptActivation { source, .. }
            | Self::LogicalScriptActivation { source, .. } => source,
        }
    }

    pub(super) fn frame(&self) -> &crate::var_resolve::VariableExecutionFrame {
        match self {
            Self::Body(body) => body.frame(),
            Self::DeclaredReceiver(body) => body.preview_frame(),
            Self::LogicalReceiver(body) => &body.frame,
            Self::DeclaredProcedure(body) => &body.frame,
            Self::RootScript { frame, .. }
            | Self::ScriptActivation { frame, .. }
            | Self::LogicalScriptActivation { frame, .. } => frame,
        }
    }

    pub(super) fn parameters(&self) -> &[tcl_syntax::formal_params::FormalParameter] {
        match self {
            Self::Body(body) => body.parameters(),
            Self::DeclaredReceiver(body) => body.parameters(),
            Self::DeclaredProcedure(body) => &body.parameters,
            Self::LogicalReceiver(_)
            | Self::RootScript { .. }
            | Self::ScriptActivation { .. }
            | Self::LogicalScriptActivation { .. } => &[],
        }
    }

    pub(super) fn original_formal_topology(
        &self,
    ) -> Option<&super::formal_topology::OriginalFormalTopology> {
        match self {
            Self::Body(body) => body.original_formal_topology(),
            Self::DeclaredReceiver(body) => body.original_formal_topology(),
            Self::DeclaredProcedure(body) => body.original_parameters.as_ref(),
            Self::LogicalReceiver(_)
            | Self::RootScript { .. }
            | Self::ScriptActivation { .. }
            | Self::LogicalScriptActivation { .. } => None,
        }
    }

    pub(super) fn owns_source(&self, origin: &Arc<super::SourceOriginId>, offset: u32) -> bool {
        let source = self.source();
        &source.origin == origin
            && offset >= source.base()
            && u64::from(offset) < u64::from(source.base()) + source.text.len() as u64
    }

    /// A readonly insertion cursor immediately before an unchanged body's
    /// closer belongs to that body's naming frame. Source occurrences remain
    /// half-open; materialised body values cannot borrow this end affinity.
    fn owns_source_cursor(&self, origin: &Arc<super::SourceOriginId>, offset: u32) -> bool {
        if self.owns_source(origin, offset) {
            return true;
        }
        let source = self.source();
        let super::ExecutedScriptMapping::Contiguous { base } = source.mapping else {
            return false;
        };
        let Some(end) = u32::try_from(source.text.len())
            .ok()
            .and_then(|length| base.checked_add(length))
        else {
            return false;
        };
        &source.origin == origin
            && offset == end
            && origin
                .source_image()
                .bytes()
                .get(base as usize..end as usize)
                == Some(source.text.bytes())
    }

    #[cfg(test)]
    pub(super) fn owns_invocation(&self, binding: &super::SourceInvocationBinding) -> bool {
        binding.variable_frame == *self.frame()
    }

    pub(super) fn owns_frame(
        &self,
        bindings: &SourceCommandBindings,
        frame: &crate::var_resolve::VariableExecutionFrame,
    ) -> bool {
        match self {
            Self::Body(body) => bindings.original_body_owns_frame(body, frame),
            Self::DeclaredReceiver(body) => body.preview_frame() == frame,
            Self::LogicalReceiver(_) => false,
            Self::DeclaredProcedure(body) => &body.frame == frame,
            Self::RootScript {
                frame: expected, ..
            }
            | Self::ScriptActivation {
                frame: expected, ..
            }
            | Self::LogicalScriptActivation {
                frame: expected, ..
            } => expected == frame,
        }
    }
}

fn original_alias_place_matches(
    row: &DeclarationLayoutObservation,
    input: &crate::signature_scan::scope::SignatureSourceNameInput,
    alias: &crate::signature_scan::variable_symbol::OriginalVariableAliasReceipt,
    place: &crate::place::Place,
) -> bool {
    use crate::signature_scan::variable_symbol::SignatureSourceVariableSlot;
    let Some(cell) = place.cell.as_ref() else {
        return false;
    };
    let crate::place::CellOwner::NamespaceIdentity(owner) = &cell.owner else {
        return false;
    };
    if place.dynamic || place.kind == crate::place::PlaceKind::Unknown {
        return false;
    }

    match alias.target().slot() {
        SignatureSourceVariableSlot::C { namespace, simple } => {
            &cell.name == simple
                && row
                    .snapshot
                    .state
                    .original_namespace_geometry(owner, input.policy())
                    == Some(crate::signature_scan::scope::SignatureNamespaceScope::C(
                        namespace.clone(),
                    ))
        }
        SignatureSourceVariableSlot::Jim(key) => &cell.name == key,
        SignatureSourceVariableSlot::Local { .. } => false,
    }
}

// Derived equality compares in field order: the whole lookup snapshot last.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeclarationLayoutObservation {
    pub(super) issuer: DeclarationLayoutIssuer,
    pub(super) config: tcl_lexer::LexerConfig,
    pub(super) words: Arc<[crate::ir::WordExpr]>,
    pub(super) namespace: super::SourceNamespaceKey,
    pub(super) entry: Arc<OriginalDiagnosticFrameEntry>,
    pub(super) snapshot: Arc<SourceLookupSnapshot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum DeclarationLayoutIssuer {
    OriginalDeclaration,
    EnteredActivation,
    /// Independent source preview, without entered receiver authority.
    ConditionalReceiverSource,
}

/// A sealed conditional source declaration whose original command has no
/// dispatch point. The saved raw binding retains unknown/absence and every
/// source/world attachment; its targets remain separate from runtime lookup.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct OriginalLogicalDeclarationTargets {
    original_binding: Box<super::SourceInvocationBinding>,
    targets: Vec<super::SourceCommandTarget>,
}

impl OriginalLogicalDeclarationTargets {
    fn matches(
        &self,
        binding: &super::SourceInvocationBinding,
        advice: &OriginalCompilationLookupAdvice,
    ) -> bool {
        let mut original = binding.clone();
        original.original_logical_declaration_targets = None;
        original.lookup_state.is_none()
            && &original == self.original_binding.as_ref()
            && self.targets.as_slice() == advice.targets()
    }
}

/// Select the immutable declaration owner without joining entered activations.
/// A conflicting original receipt withdraws the entire declaration projection.
pub(super) fn original_declaration_layouts(
    observations: &[DeclarationLayoutObservation],
) -> Option<impl Iterator<Item = &DeclarationLayoutObservation> + Clone> {
    select_declaration_layouts(observations, false)
}

/// Catalogue source advice also admits independently collected receiver bodies.
/// Those rows cannot supply the original execution or read projection.
pub(super) fn original_source_catalogue_layouts(
    observations: &[DeclarationLayoutObservation],
) -> Option<impl Iterator<Item = &DeclarationLayoutObservation> + Clone> {
    select_declaration_layouts(observations, true)
}

fn select_declaration_layouts(
    observations: &[DeclarationLayoutObservation],
    catalogue: bool,
) -> Option<impl Iterator<Item = &DeclarationLayoutObservation> + Clone> {
    let original = observations.iter().filter(move |observation| {
        observation.issuer == DeclarationLayoutIssuer::OriginalDeclaration
            || (catalogue
                && observation.issuer == DeclarationLayoutIssuer::ConditionalReceiverSource)
    });
    #[cfg(debug_assertions)]
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_DECLARATION").is_some()
        && original.clone().next().is_none()
    {
        eprintln!(
            "ORIGINAL_DECLARATION stage=no-original observations={}",
            observations.len()
        );
    }
    let first = super::trace_rbs_operand_option("rows.first_original", original.clone().next())?;
    #[cfg(debug_assertions)]
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_DECLARATION").is_some() {
        for observation in original.clone() {
            eprintln!(
                "ORIGINAL_DECLARATION offset={} stage=owner entry={} config={} words={} frame={} origin={} context={}",
                observation
                    .words
                    .first()
                    .map_or(0, |word| word.source().span.start()),
                observation.entry == first.entry,
                observation.config == first.config,
                observation.words == first.words,
                observation.snapshot.state.variable_frame == *observation.entry.frame(),
                observation.snapshot.state.current_source_origin.as_ref()
                    == Some(&observation.entry.source().origin),
                observation
                    .entry
                    .owns_original_context(&observation.snapshot.state.source_variables),
            );
        }
    }
    if original.clone().any(|observation| {
        // The first row is equal to itself; only later rows need the join.
        (!std::ptr::eq(observation, first)
            && (observation.entry != first.entry
                || observation.config != first.config
                || observation.words != first.words))
            || observation.snapshot.state.current_source_origin.as_ref()
                != Some(&observation.entry.source().origin)
            || (catalogue
                && observation.words.first().is_none_or(|word| {
                    !observation.entry.owns_source(
                        &observation.entry.source().origin,
                        word.source().span.start(),
                    )
                }))
            || (!catalogue
                && (observation.snapshot.state.variable_frame != *observation.entry.frame()
                    || !observation
                        .entry
                        .owns_original_context(&observation.snapshot.state.source_variables)))
    }) {
        return super::trace_rbs_operand_option("rows.owner_join", None);
    }
    Some(original)
}

// Snapshot hashing is cached. Omitting the declaration recipe permits harmless
// hash collisions; structural equality still compares the complete owner.
impl Hash for DeclarationLayoutObservation {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.issuer.hash(state);
        self.snapshot.hash(state);
        self.namespace.hash(state);
        self.config.hash(state);
        self.words.hash(state);
    }
}

pub(super) type DeclarationLayouts =
    BTreeMap<CommandAllocationSite, Vec<DeclarationLayoutObservation>>;

fn original_local_primary_is_byte_unique(
    state: &ModuleCommandBindings,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    name: &[u8],
) -> bool {
    let options = super::SourceAnalysisOptions {
        native_entry: state.baseline.native_entry.as_deref(),
        invocation_dialect: state.baseline.dialect,
        compiled_variable_provider: state.baseline.compiled_variable_provider,
        ..Default::default()
    };
    if options.execution_name_policy()
        != Some(tcl_syntax::naming::ExecutionNamePolicy::NativeRecipe(
            policy,
        ))
    {
        return false;
    }
    if policy.recipe().is_jim084() {
        // Jim installs formals through its selected general setter. It has no
        // C compiled-local primary to donate; the caller still checks the
        // exact direct scalar installation and any current alias separately.
        return true;
    }
    options
        .compiled_variable_protocol()
        .is_some_and(|compiler| {
            compiler.compiled_local_name_is_byte_unique(name)
                && state
                    .source_variables
                    .original_formal_topology
                    .as_ref()
                    .and_then(|topology| topology.first_compiled_formal_name(name, compiler, false))
                    .is_none_or(|primary| primary == name)
        })
}

fn original_alias_template_input_is_current(
    row: &DeclarationLayoutObservation,
    input: &crate::signature_scan::scope::SignatureSourceNameInput,
) -> bool {
    let Some(owner) = row.snapshot.state.logical_source_name_advice_input() else {
        return input.is_current(&row.snapshot.state.source_variables);
    };
    if !owner.has_logical_source_name_context()
        || owner.lexer_config() != row.config
        || row.snapshot.state.baseline.native_entry.is_some()
        || super::source_analysis_entry::source_input_dialect(owner).authored_name_policy()
            != Some(input.policy())
    {
        return false;
    }
    let image = row.entry.source().origin.source_image();
    match input {
        crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(word) => {
            word.source_image() == image && word.lexer_config() == row.config
        }
        crate::signature_scan::scope::SignatureSourceNameInput::OriginalVariableRoot(root) => {
            root.source_image() == image && root.lexer_config() == row.config
        }
        crate::signature_scan::scope::SignatureSourceNameInput::OriginalValue(_)
        | crate::signature_scan::scope::SignatureSourceNameInput::OriginalLogicalListField(_) => {
            false
        }
    }
}

fn original_alias_template_local_is_byte_unique(
    row: &DeclarationLayoutObservation,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    name: &[u8],
) -> bool {
    let state = &row.snapshot.state;
    if let Some(input) = state.logical_source_name_advice_input() {
        if !input.has_logical_source_name_context()
            || input.lexer_config() != row.config
            || state.baseline.native_entry.is_some()
            || state.variable_frame != *row.entry.frame()
            || !row.entry.owns_original_context(&state.source_variables)
        {
            return false;
        }
        return match row.entry.as_ref() {
            OriginalDiagnosticFrameEntry::DeclaredProcedure(body) => {
                body.logical_alias_local_name_is_byte_unique(policy, row.config)
            }
            _ => false,
        };
    }
    original_local_primary_is_byte_unique(state, policy, name)
}

fn original_alias_template_rows_match<'a>(
    rows: impl Iterator<Item = &'a DeclarationLayoutObservation>,
    input: &crate::signature_scan::scope::SignatureSourceNameInput,
    receiver: crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver,
    alias: &crate::signature_scan::variable_symbol::OriginalVariableAliasReceipt,
    requires_qualified_target: impl Fn() -> bool,
    absolute_root: impl Fn() -> bool,
) -> Option<bool> {
    let mut found = false;
    for row in rows {
        if !original_alias_template_input_is_current(row, input)
            || !original_alias_template_local_is_byte_unique(
                row,
                input.policy(),
                alias.local().as_bytes(),
            )
        {
            return None;
        }
        if requires_qualified_target() {
            let namespace = if absolute_root() {
                Some(crate::signature_scan::scope::SignatureNamespaceScope::root(
                    Some(input.policy()),
                ))
            } else {
                row.snapshot
                    .state
                    .original_namespace_geometry(&row.namespace, input.policy())
            }?;
            let target = crate::signature_scan::variable_symbol::SignatureSourceVariableSymbol::from_original_input(
                input,
                &namespace,
                crate::signature_scan::variable_symbol::OriginalVariableSymbolPurpose::QualifiedOnly,
                receiver,
            );
            if target.as_ref() != Some(alias.target()) {
                return None;
            }
        }
        found = true;
    }
    Some(found)
}

/// Immutable original local-frame naming owner. This is a conditional source
/// declaration identity, not an entered activation or a physical local table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceOriginalVariableFrame {
    source: super::ExecutedScriptSource,
    frame: crate::var_resolve::VariableExecutionFrame,
    config: tcl_lexer::LexerConfig,
    logical_lambda: Option<Arc<super::OriginalLogicalLambdaBody>>,
}

impl std::hash::Hash for SourceOriginalVariableFrame {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.source, state);
        std::hash::Hash::hash(&self.frame, state);
        std::hash::Hash::hash(&self.config, state);
    }
}

impl SourceOriginalVariableFrame {
    // Conditional authored procedure layout only; no entered frame or cell.
    pub(crate) fn from_authored_scalar_source(
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Self> {
        let binding = tokens.source_binding.as_ref()?;
        let scope = binding.original_authored_scalar_body_scope(tokens, registry)?;
        scope.original_words(tokens, registry)?;
        scope.scalar_name_policy()?;
        let advice = binding.original_authored_declaration_scalar_layout(tokens, registry)?;
        if advice.has_unclosed_source_coverage()
            || scope.has_unclosed_declaration_source_coverage()
            || advice.namespace() != scope.namespace()
        {
            return None;
        }
        let body = scope.scalar_frame_body();
        let span = body.content_span().ok()?;
        let origin = &binding.invocation_site()?.source;
        (origin.source_image() == body.image()).then_some(())?;
        let text = body.image().try_text().ok()?.get(span.as_range())?;
        Some(Self {
            source: super::ExecutedScriptSource::contiguous_image(
                Arc::clone(origin),
                tcl_lexer::SourceImage::document(text),
                span.start(),
            )?,
            frame: advice.frame().clone(),
            config: scope.config(),
            logical_lambda: None,
        })
    }

    /// A release unit's procedure body frame under its authored simulation:
    /// the procedure's executed body under its own namespace, with the
    /// activation identity the walk gives the declaration's frame. The formal
    /// list must parse strictly to the declaration's own parameters, each a
    /// unique plain local. Conditional layout only; no entered frame or cell.
    pub(crate) fn from_authored_procedure(
        procedure: &crate::ir::Procedure,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Self> {
        use tcl_syntax::naming::{NamePolicyAuthority, NativeNameProtocol};
        let policy =
            super::source_analysis_entry::source_input_dialect(input).authored_name_policy()?;
        if policy.authority() != NamePolicyAuthority::AuthoredSimulation
            || !matches!(policy.recipe(), NativeNameProtocol::C(_))
        {
            return None;
        }
        let parameters = super::logical_definition::static_formals(
            &procedure.params_raw,
            input.lexer_config().list_parse,
        )?;
        if !parameters
            .iter()
            .map(|formal| formal.name.as_str())
            .eq(procedure.params.iter().map(String::as_str))
            || !super::logical_definition::local_formals_are_unique(&parameters, policy)
        {
            return None;
        }
        let source = procedure.body.executed_source.as_deref()?.clone();
        let namespace = procedure.body.namespace_context.as_deref()?.clone();
        let site = procedure.declaration_site.as_ref()?;
        Some(Self {
            source,
            frame: crate::var_resolve::VariableExecutionFrame::Procedure {
                namespace: namespace.display()?,
                identity: super::source_activation_name(
                    Some(&site.source),
                    &procedure.qualified_name,
                    site.offset,
                ),
            }
            .with_namespace_identity(namespace),
            config: input.lexer_config(),
            logical_lambda: None,
        })
    }

    /// Conditional lexical lambda layout from its held call and source fields.
    /// This does not issue an entered frame, installed local or Native recipe.
    pub(crate) fn from_logical_lambda_source(
        receipt: Arc<super::OriginalLogicalLambdaBody>,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Self> {
        let (origin, offset, body, namespace) = receipt.frame_source(input)?;
        let crate::signature_scan::scope::SignatureNamespaceScope::C(path) = namespace else {
            return None;
        };
        let namespace = super::SourceNamespaceKey::authored(
            tcl_syntax::naming::checked_namespace_path_key(&path)?,
        );
        let text = origin
            .source_image()
            .try_text()
            .ok()?
            .get(body.as_range())?;
        let source = super::ExecutedScriptSource::contiguous_image(
            Arc::clone(&origin),
            tcl_lexer::SourceImage::document(text),
            body.start(),
        )?;
        Some(Self {
            source,
            frame: crate::var_resolve::VariableExecutionFrame::Procedure {
                namespace: namespace.display()?,
                identity: super::source_activation_name(
                    Some(&origin),
                    "logical-lambda-layout",
                    offset,
                ),
            }
            .with_namespace_identity(namespace),
            config: input.lexer_config(),
            logical_lambda: Some(receipt),
        })
    }

    pub(crate) fn from_logical_procedure_source(
        card: &super::OriginalLogicalProcedureBody,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Self> {
        let image = card.site().source.source_image();
        if !super::source_analysis_entry::source_input_dialect(input)
            .authored_name_policy()
            .is_some_and(|policy| {
                policy.authority() == tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
                    && matches!(
                        policy.recipe(),
                        tcl_syntax::naming::NativeNameProtocol::C(_)
                    )
            })
            || !card.matches_source(image, input)
            || card.scope().frame_recipe() != super::OriginalDeclaredSourceBodyFrame::Procedure
        {
            return None;
        }
        let body = card.scope().scope_body().content_span();
        let text = image.try_text().ok()?.get(body.as_range())?;
        let source = super::ExecutedScriptSource::contiguous_image(
            Arc::clone(&card.site().source),
            tcl_lexer::SourceImage::document(text),
            body.start(),
        )?;
        let namespace = super::SourceNamespaceKey::authored(
            tcl_syntax::naming::checked_namespace_path_key(card.scope().namespace_recipe())?,
        );
        Some(Self {
            source,
            frame: super::declaration_preview::declared_procedure_frame(card.site(), &namespace),
            config: input.lexer_config(),
            logical_lambda: None,
        })
    }

    pub(super) fn from_logical_namespace_source(
        scope: &super::OriginalDeclaredLogicalBodyContext,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Self> {
        if scope.frame_recipe() != super::OriginalDeclaredSourceBodyFrame::Namespace
            || !scope.matches_input(input)
            || !super::source_analysis_entry::source_input_dialect(input)
                .authored_name_policy()
                .is_some_and(|policy| {
                    policy.authority()
                        == tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
                        && matches!(
                            policy.recipe(),
                            tcl_syntax::naming::NativeNameProtocol::C(_)
                        )
                })
        {
            return None;
        }
        let crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(
            advice,
        ) = scope.scope_body().source_words().source()
        else {
            return None;
        };
        if advice.logical_source_input() != Some(input) {
            return None;
        }
        let origin = &advice.site().source;
        let image = origin.source_image();
        let body = scope.scope_body().content_span();
        let text = image.try_text().ok()?.get(body.as_range())?;
        let namespace = tcl_syntax::naming::checked_namespace_path_key(scope.namespace_recipe())?;
        Some(Self {
            source: super::ExecutedScriptSource::contiguous_image(
                Arc::clone(origin),
                tcl_lexer::SourceImage::document(text),
                body.start(),
            )?,
            frame: crate::var_resolve::VariableExecutionFrame::NamespaceActivation {
                namespace,
                identity: format!(
                    "logical-namespace-source:{}:{}",
                    super::SourceOriginId::activation_key(origin),
                    body.start()
                ),
            },
            config: input.lexer_config(),
            logical_lambda: None,
        })
    }

    /// Direct cursor membership in an independently selected same-frame source
    /// region. The caller retains its scope/frame recipe; this checks only the
    /// actual region owner and its containment in this held frame source.
    pub(crate) fn source_region_cursor_is_direct(
        &self,
        image: &tcl_lexer::SourceImage,
        region: &crate::registry_invocation::OriginalSourceScriptBody,
        offset: u32,
    ) -> bool {
        let crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(
            advice,
        ) = region.source_words().source()
        else {
            return false;
        };
        let span = region.content_span();
        let Some(end) = u32::try_from(self.source.text.bytes().len())
            .ok()
            .and_then(|len| self.source.base().checked_add(len))
        else {
            return false;
        };
        Arc::ptr_eq(&advice.site().source, &self.source.origin)
            && region.matches_source(image, self.config)
            && self.source.origin.source_image() == image
            && self.source.base() <= span.start()
            && span.end() <= end
            && span.start() <= offset
            && offset <= span.end()
            && direct_script_cursor(image, self.config, span, offset, 0)
    }

    /// Conditional procedure, method or lambda layout selected by its source
    /// body recipe. This supplies no entered activation or physical local table.
    #[must_use]
    pub const fn frame(&self) -> &crate::var_resolve::VariableExecutionFrame {
        &self.frame
    }

    /// Complete retained source owner and its truthful body mapping.
    #[must_use]
    pub const fn source(&self) -> &super::ExecutedScriptSource {
        &self.source
    }

    /// Original full parser configuration retained with the frame recipe.
    #[must_use]
    pub const fn lexer_config(&self) -> tcl_lexer::LexerConfig {
        self.config
    }
}

/// A braced operand without its own selected body receipt cannot inherit the
/// enclosing procedure's locals. Command substitutions retain the same frame.
fn direct_script_cursor(
    image: &tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    span: tcl_lexer::Span,
    offset: u32,
    depth: usize,
) -> bool {
    if depth > 64 {
        return false;
    }
    let Ok(plan) = tcl_lexer::native_script_words_in(image.clone(), span, config) else {
        return false;
    };
    if plan.fatal_tail.is_some() {
        return false;
    }
    for command in plan.commands {
        for word in command.words {
            if word.group().kind == tcl_lexer::WordKind::Braced
                && word
                    .content_span()
                    .is_ok_and(|content| content.start() <= offset && offset <= content.end())
            {
                return false;
            }
            let arena = word.executable_parts();
            for part in arena.all_parts() {
                if let tcl_lexer::ExecutablePart::Command { body } = part.part
                    && body.start() <= offset
                    && offset <= body.end()
                {
                    return direct_script_cursor(image, config, body, offset, depth + 1);
                }
            }
        }
    }
    true
}

impl SourceCommandBindings {
    /// Original Logical receiver-body frame geometry for source navigation.
    /// Only the retained member/body/vector and actual parent declaration may
    /// supply this recipe. Native entry and receiver namespace stay unavailable.
    pub(crate) fn logical_source_member_frame_at_offset(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        image: &tcl_lexer::SourceImage,
        offset: u32,
        config: tcl_lexer::LexerConfig,
    ) -> Option<SourceOriginalVariableFrame> {
        if !input.has_logical_source_name_context()
            || input.lexer_config() != config
            || self.original_logical_source_name_advice_input() != Some(input)
            || !self.matches_original_source_image(image, config)
        {
            return None;
        }
        let origin = self.source_origin()?;
        let mut selected: Option<SourceOriginalVariableFrame> = None;
        for (site, rows) in &self.declaration_layouts {
            let Some(rows) = original_source_catalogue_layouts(rows) else {
                continue;
            };
            let first = rows.clone().next()?;
            let OriginalDiagnosticFrameEntry::LogicalReceiver(body) = first.entry.as_ref() else {
                continue;
            };
            if body.input != *input
                || !Arc::ptr_eq(&body.source.origin, origin)
                || !body.body.matches_source(image, config)
                || !body.body.matches_context(input.borrowed_context_registry())
                || !first.entry.owns_source_cursor(origin, offset)
                || !direct_script_cursor(image, config, body.body.content_span(), offset, 0)
            {
                continue;
            }
            let crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(parent) =
                body.body.original_parent_body().source_words().source()
            else {
                return None;
            };
            let parents =
                original_declaration_layouts(self.declaration_layouts.get(parent.site())?)?;
            if parent.logical_source_input() != Some(input)
                || !Arc::ptr_eq(&parent.site().source, origin)
                || rows.clone().any(|row| {
                    row.issuer != DeclarationLayoutIssuer::ConditionalReceiverSource
                        || row.config != config
                        || row.snapshot.state.logical_source_name_advice_input() != Some(input)
                        || !parents.clone().any(|parent| {
                            parent.snapshot == row.snapshot && parent.namespace == row.namespace
                        })
                        || origin
                            .original_lexical_command(&row.words, site.offset, config)
                            .is_none_or(|original| {
                                original.native_words().iter().any(|word| {
                                    word.span().start() < body.body.content_span().start()
                                        || word.span().end() > body.body.content_span().end()
                                })
                            })
                })
            {
                return None;
            }
            let frame = SourceOriginalVariableFrame {
                source: body.source.clone(),
                frame: body.frame.clone(),
                config,
                logical_lambda: None,
            };
            match &selected {
                Some(previous) if previous.source.text.len() < frame.source.text.len() => {}
                Some(previous) if previous.source.text.len() == frame.source.text.len() => {
                    if previous != &frame {
                        return None;
                    }
                }
                _ => selected = Some(frame),
            }
        }
        selected
    }

    pub(super) fn retain_original_script_activation(
        &mut self,
        image: &tcl_lexer::SourceImage,
        base: u32,
        state: &ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) {
        if &state.variable_frame != context.frame {
            return;
        }
        let Some(origin) = state.current_source_origin.as_ref() else {
            return;
        };
        let namespace = context.namespace_identity();
        if state.source_variables.namespace_identity.as_ref() != Some(&namespace) {
            return;
        }
        // A nested parser does not issue a frame. Reborrow only a genuine
        // entered operand observation carrying this exact source and frame.
        let entered = (context.source_purpose()
            == tcl_lexer::SourceSegmentationPurpose::NestedScript)
            .then(|| {
                self.entered_scripts
                    .values()
                    .flat_map(|arguments| arguments.values().flatten())
                    .find(|observation| {
                        observation.source.origin == *origin
                            && observation.source.text == *image
                            && observation.source.base() == base
                            && observation.frame.as_ref() == Some(context.frame)
                            && observation.namespace.as_ref() == Some(&namespace)
                    })
                    .map(|observation| observation.source.clone())
            })
            .flatten();
        let source = if let Some(source) = entered {
            source
        } else {
            if self.root_origin.as_ref() != Some(origin)
                || !matches!(
                    state.variable_frame.layout(),
                    crate::var_resolve::VariableExecutionFrame::NamespaceActivation { .. }
                )
            {
                return;
            }
            let Some(source) = super::ExecutedScriptSource::contiguous_image(
                Arc::clone(origin),
                image.clone(),
                base,
            ) else {
                return;
            };
            source
        };
        let logical = context.body_entry() == super::SourceBodyEntryPurpose::Conditional
            && state.baseline.logical_source_input.is_some();
        if logical && super::logical_definition::logical_entry(state, *context).is_none() {
            return;
        }
        let entry = Arc::new(if logical {
            OriginalDiagnosticFrameEntry::LogicalScriptActivation {
                source,
                frame: state.variable_frame.clone(),
                namespace,
                config: context.config,
            }
        } else {
            OriginalDiagnosticFrameEntry::ScriptActivation {
                source,
                frame: state.variable_frame.clone(),
                namespace,
                config: context.config,
            }
        });
        if !self.original_script_activations.contains(&entry) {
            self.original_script_activations.push(entry);
        }
    }

    fn original_script_activation_at(
        &self,
        state: &ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        origin: &Arc<super::SourceOriginId>,
        offset: u32,
    ) -> Option<OriginalDiagnosticFrameEntry> {
        let namespace = context.namespace_identity();
        let mut matching = self.original_script_activations.iter().filter(|entry| {
            matches!(entry.as_ref(), OriginalDiagnosticFrameEntry::ScriptActivation {
                frame, namespace: selected, config, ..
            } | OriginalDiagnosticFrameEntry::LogicalScriptActivation {
                frame, namespace: selected, config, ..
            } if frame == &state.variable_frame && frame == context.frame && selected == &namespace && *config == context.config)
                && (matches!(entry.as_ref(), OriginalDiagnosticFrameEntry::LogicalScriptActivation { .. })
                    == (context.body_entry() == super::SourceBodyEntryPurpose::Conditional
                        && super::logical_definition::logical_entry(state, *context).is_some()))
                && entry.owns_source(origin, offset)
                && entry.owns_original_context(&state.source_variables)
        });
        let first = matching.next()?;
        matching
            .all(|entry| entry == first)
            .then(|| first.as_ref().clone())
    }

    pub(crate) fn original_namespace_variable_symbol_at_span(
        &self,
        span: tcl_lexer::Span,
        input: &crate::signature_scan::scope::SignatureSourceNameInput,
        receiver: crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver,
        config: tcl_lexer::LexerConfig,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<(
        crate::signature_scan::scope::SignatureNamespaceScope,
        tcl_core_types::NameBytes,
    )> {
        if !matches!(
            receiver,
            crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver::LexicalRoot
                | crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver::Operand(
                    _
                )
        ) {
            return None;
        }
        let form = receiver.input_form(input)?;
        let origin = self.root_origin.as_ref()?;
        let frame = self.original_variable_frame_at_span(span, config);
        let mut selected = None;
        for (site, rows) in &self.declaration_layouts {
            if &site.source != origin || site.offset > span.start() {
                continue;
            }
            let Some(rows) = original_declaration_layouts(rows) else {
                continue;
            };
            let first = rows.clone().next()?;
            if first.config != config
                || !first.entry.owns_source(origin, span.start())
                || frame.as_ref().is_some_and(|frame| {
                    first.entry.frame() != frame.frame() || first.entry.source() != frame.source()
                })
                || !first.words.iter().any(|word| {
                    word.source().span.start() <= span.start()
                        && span.end() <= word.source().span.end()
                })
            {
                continue;
            }
            for row in rows {
                let state = &row.snapshot.state;
                let context = &state.source_variables;
                input.is_current(context).then_some(())?;
                let place = crate::var_resolve::resolve_evaluated_variable_input(
                    form,
                    context,
                    false,
                    registry,
                    tcl_registry::TraceOperation::Read,
                );
                if place.dynamic || place.kind == crate::place::PlaceKind::Unknown {
                    return None;
                }
                let cell = place.cell.as_ref()?;
                let crate::place::CellOwner::NamespaceIdentity(owner) = &cell.owner else {
                    return None;
                };
                let value = (
                    state.original_namespace_geometry(owner, input.policy())?,
                    cell.name.clone(),
                );
                if selected.as_ref().is_some_and(|previous| previous != &value) {
                    return None;
                }
                selected = Some(value);
            }
        }
        selected
    }

    /// Original local root selected unanimously in the declaration's own
    /// point. Aliases and unknown bindings cannot borrow the displayed scope.
    pub(crate) fn original_local_variable_symbol_at_span(
        &self,
        span: tcl_lexer::Span,
        input: &crate::signature_scan::scope::SignatureSourceNameInput,
        receiver: crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver,
        config: tcl_lexer::LexerConfig,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<(SourceOriginalVariableFrame, tcl_core_types::NameBytes)> {
        use tcl_syntax::naming::{
            NativeNameContext, NativeVariableInputForm, NativeVariableRootGeometry,
        };
        let frame = self.original_variable_frame_at_span(span, config);
        #[cfg(test)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_SYMBOLS").is_some() {
            eprintln!(
                "ORIGINAL_VARIABLE_LOCAL span={span:?} bytes={:?} frame={} rows={}",
                input.bytes(),
                frame.is_some(),
                self.declaration_layouts.len()
            );
        }
        let frame = frame?;
        let protocol = input.policy().recipe();
        if !matches!(
            receiver,
            crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver::LexicalRoot
                | crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver::Operand(
                    _
                )
        ) {
            return None;
        }
        let form = receiver.input_form(input)?;
        let selected = match form {
            NativeVariableInputForm::Combined(bytes) => protocol.combined_variable_input(bytes),
            NativeVariableInputForm::Separate { root, element } => {
                protocol.separate_variable_input(root, element)
            }
        };
        let NativeVariableRootGeometry::Local(simple) =
            protocol.variable_root_geometry(NativeNameContext::root(), selected.root().selected())
        else {
            return None;
        };
        let origin = self.root_origin.as_ref()?;
        let mut found = false;
        for (site, rows) in self.root_layouts_spanning(span.start(), span.end()) {
            if &site.source != origin || site.offset > span.start() {
                continue;
            }
            let Some(rows) = original_declaration_layouts(rows) else {
                continue;
            };
            let first = rows.clone().next()?;
            if first.config != config
                || first.entry.frame() != frame.frame()
                || first.entry.source() != frame.source()
                || !first.words.iter().any(|word| {
                    word.source().span.start() <= span.start()
                        && span.end() <= word.source().span.end()
                })
            {
                continue;
            }
            for row in rows {
                let state = &row.snapshot.state;
                let context = &state.source_variables;
                if !input.is_current(context) {
                    return None;
                }
                // The lexical/compiler and runtime projections must select
                // this same exact primary. A raw-NUL LVT match cannot use this
                // deliberately value-free uniform naming projection.
                if !original_local_primary_is_byte_unique(state, input.policy(), simple.as_bytes())
                {
                    return None;
                }
                let place = crate::var_resolve::resolve_evaluated_variable_input(
                    form,
                    context,
                    false,
                    registry,
                    tcl_registry::TraceOperation::Read,
                );
                let cell = place.cell.as_ref()?;
                #[cfg(test)]
                if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_SYMBOLS")
                    .is_some()
                {
                    eprintln!(
                        "ORIGINAL_VARIABLE_LOCAL_POINT span={span:?} site={} kind={:?} dynamic={} owner={:?} name={:?} activation={:?} bindings_unknown={}",
                        site.offset,
                        place.kind,
                        place.dynamic,
                        cell.owner,
                        cell.name,
                        context.activation,
                        context.dynamic_bindings
                    );
                }
                if place.dynamic
                    || place.kind == crate::place::PlaceKind::Unknown
                    || !matches!(&cell.owner, crate::place::CellOwner::Activation(activation) if context.activation.as_ref() == Some(activation))
                    || cell.name != simple
                {
                    return None;
                }
                found = true;
            }
        }
        found.then_some((frame, simple))
    }

    #[cfg(test)]
    pub(crate) fn original_variable_uses_global_frame_at_span(
        &self,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        let Some(origin) = &self.root_origin else {
            return false;
        };
        let mut found = false;
        for (site, rows) in &self.declaration_layouts {
            if &site.source != origin || site.offset > span.start() {
                continue;
            }
            let Some(rows) = original_declaration_layouts(rows) else {
                continue;
            };
            let Some(first) = rows.clone().next() else {
                continue;
            };
            if first.config != config
                || !first.entry.owns_source(origin, span.start())
                || !first.words.iter().any(|word| {
                    word.source().span.start() <= span.start()
                        && span.end() <= word.source().span.end()
                })
            {
                continue;
            }
            if rows.clone().any(|row| {
                row.entry.frame().layout() != &crate::var_resolve::VariableExecutionFrame::Global
            }) {
                return false;
            }
            found = true;
        }
        found
    }

    pub(crate) fn original_variable_alias_frame_at_span(
        &self,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
    ) -> Option<(SourceOriginalVariableFrame, u32)> {
        let frame = self.original_variable_frame_at_span(span, config)?;
        let origin = self.root_origin.as_ref()?;
        let mut found = None;
        for (site, rows) in &self.declaration_layouts {
            if &site.source != origin || site.offset > span.start() {
                continue;
            }
            let Some(rows) = original_declaration_layouts(rows) else {
                continue;
            };
            let first = rows.clone().next()?;
            if first.config == config
                && first.entry.frame() == frame.frame()
                && first.entry.source() == frame.source()
                && first.words.iter().any(|word| word.source().span == span)
            {
                if found.is_some_and(|offset| offset != site.offset) {
                    return None;
                }
                found = Some(site.offset);
            }
        }
        Some((frame, found?))
    }

    /// Original local spelling and declaration frame for a conditional source
    /// alias template. Physical-cell matching remains a separate operation.
    pub(crate) fn original_variable_alias_template_matches_at_span(
        &self,
        span: tcl_lexer::Span,
        input: &crate::signature_scan::scope::SignatureSourceNameInput,
        receiver: crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver,
        alias: &crate::signature_scan::variable_symbol::OriginalVariableAliasReceipt,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        use tcl_syntax::naming::{
            NativeNameContext, NativeVariableInputForm, NativeVariableRootGeometry,
        };
        if input.policy() != alias.target().policy() || alias.frame().lexer_config() != config {
            return false;
        }
        let Some(form) = receiver.input_form(input) else {
            return false;
        };
        let protocol = input.policy().recipe();
        let selected = match form {
            NativeVariableInputForm::Combined(bytes) => protocol.combined_variable_input(bytes),
            NativeVariableInputForm::Separate { root, element } => {
                protocol.separate_variable_input(root, element)
            }
        };
        let declaration =
            crate::signature_scan::variable_symbol::OriginalVariableAliasAdvice::from_receipt(
                alias,
            );
        let geometry =
            protocol.variable_root_geometry(NativeNameContext::root(), selected.root().selected());
        let local =
            matches!(&geometry, NativeVariableRootGeometry::Local(name) if name == alias.local());
        if span != declaration.span()
            && !local
            && matches!(geometry, NativeVariableRootGeometry::Local(_))
        {
            return false;
        }
        let Some(frame) = self.original_variable_frame_at_span(span, config) else {
            return false;
        };
        if &frame != alias.frame() {
            return false;
        }
        let Some(origin) = &self.root_origin else {
            return false;
        };
        let mut found = false;
        for (site, observations) in &self.declaration_layouts {
            if &site.source != origin
                || site.offset < alias.invocation_offset()
                || site.offset > span.start()
            {
                continue;
            }
            let Some(rows) = original_declaration_layouts(observations) else {
                continue;
            };
            let Some(first) = rows.clone().next() else {
                continue;
            };
            if first.config != config
                || first.entry.frame() != frame.frame()
                || first.entry.source() != frame.source()
                || !first.words.iter().any(|word| {
                    word.source().span.start() <= span.start()
                        && span.end() <= word.source().span.end()
                })
            {
                continue;
            }
            let Some(site_found) = original_alias_template_rows_match(
                rows,
                input,
                receiver,
                alias,
                || span != declaration.span() && !local,
                || {
                    selected.root().qualification()
                        == tcl_syntax::naming::NativeNameQualification::Absolute
                },
            ) else {
                return false;
            };
            found |= site_found;
        }
        found
    }

    pub(crate) fn original_variable_alias_matches_at_span(
        &self,
        span: tcl_lexer::Span,
        input: &crate::signature_scan::scope::SignatureSourceNameInput,
        receiver: crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver,
        alias: &crate::signature_scan::variable_symbol::OriginalVariableAliasReceipt,
        config: tcl_lexer::LexerConfig,
        registry: &tcl_registry::CommandRegistry,
    ) -> bool {
        use tcl_syntax::naming::{
            NativeNameContext, NativeVariableInputForm, NativeVariableRootGeometry,
        };
        let protocol = input.policy().recipe();
        if input.policy() != alias.target().policy() || alias.frame().lexer_config() != config {
            return false;
        }
        if !matches!(
            receiver,
            crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver::LexicalRoot
                | crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver::Operand(
                    _
                )
        ) {
            return false;
        }
        let Some(form) = receiver.input_form(input) else {
            return false;
        };
        let projection = match form {
            NativeVariableInputForm::Combined(bytes) => protocol.combined_variable_input(bytes),
            NativeVariableInputForm::Separate { root, element } => {
                protocol.separate_variable_input(root, element)
            }
        };
        if !matches!(protocol.variable_root_geometry(NativeNameContext::root(), projection.root().selected()), NativeVariableRootGeometry::Local(ref name) if name == alias.local())
        {
            return false;
        }
        let Some(origin) = &self.root_origin else {
            return false;
        };
        let mut found = false;
        for (site, rows) in &self.declaration_layouts {
            if &site.source != origin
                || site.offset < alias.invocation_offset()
                || site.offset > span.start()
            {
                continue;
            }
            let Some(rows) = original_declaration_layouts(rows) else {
                continue;
            };
            let Some(first) = rows.clone().next() else {
                continue;
            };
            if first.config != config
                || first.entry.frame() != alias.frame().frame()
                || first.entry.source() != alias.frame().source()
                || !first.words.iter().any(|word| {
                    word.source().span.start() <= span.start()
                        && span.end() <= word.source().span.end()
                })
            {
                continue;
            }
            for row in rows {
                let context = &row.snapshot.state.source_variables;
                if !input.is_current(context) {
                    return false;
                }
                if !original_local_primary_is_byte_unique(
                    &row.snapshot.state,
                    input.policy(),
                    alias.local().as_bytes(),
                ) {
                    return false;
                }
                let place = crate::var_resolve::resolve_evaluated_variable_input(
                    form,
                    context,
                    false,
                    registry,
                    tcl_registry::TraceOperation::Read,
                );
                if place.cell.is_none() {
                    #[cfg(test)]
                    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_SYMBOLS")
                        .is_some()
                    {
                        eprintln!(
                            "ORIGINAL_VARIABLE_ALIAS_POINT span={span:?} producer={} site={} local={:?} kind={:?} dynamic={} bindings_unknown={} missing_cell",
                            alias.invocation_offset(),
                            site.offset,
                            alias.local(),
                            place.kind,
                            place.dynamic,
                            context.dynamic_bindings
                        );
                    }
                    return false;
                }
                if !original_alias_place_matches(row, input, alias, &place) {
                    return false;
                }
                found = true;
            }
        }
        found
    }

    pub(crate) fn original_variable_formal_occurrences(
        &self,
        config: tcl_lexer::LexerConfig,
    ) -> Vec<crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence> {
        use crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence;
        let Some(origin) = &self.root_origin else {
            return Vec::new();
        };
        let mut result = Vec::new();
        let mut seen = Vec::new();
        for rows in self.declaration_layouts.values() {
            let Some(rows) = original_declaration_layouts(rows) else {
                continue;
            };
            let Some(row) = rows.clone().next() else {
                continue;
            };
            if row.config != config
                || &row.entry.source().origin != origin
                || !matches!(
                    row.entry.frame().layout(),
                    crate::var_resolve::VariableExecutionFrame::Procedure { .. }
                        | crate::var_resolve::VariableExecutionFrame::ReceiverMethod { .. }
                )
            {
                continue;
            }
            if seen.contains(&row.entry) {
                continue;
            }
            seen.push(row.entry.clone());
            let Some(topology) = row.entry.original_formal_topology() else {
                continue;
            };
            let Some(fields) = topology.original_name_fields() else {
                continue;
            };
            let frame = SourceOriginalVariableFrame {
                source: row.entry.source().clone(),
                frame: row.entry.frame().clone(),
                config,
                logical_lambda: None,
            };
            for field in fields {
                let agrees = rows.clone().all(|row| {
                    original_local_primary_is_byte_unique(
                        &row.snapshot.state,
                        field.input.policy(),
                        field.name.as_bytes(),
                    ) && field.input.is_current(&row.snapshot.state.source_variables)
                });
                if !agrees {
                    continue;
                }
                result.push(
                    SignatureSourceVariableOccurrence::from_original_formal_input(
                        field.span,
                        field.input,
                        frame.clone(),
                        field.name,
                        field.recipe,
                    ),
                );
            }
        }
        result
    }

    pub(crate) fn original_variable_formal_name_is_unrepresented(
        &self,
        symbol: &crate::signature_scan::variable_symbol::SignatureSourceVariableSymbol,
    ) -> bool {
        let crate::signature_scan::variable_symbol::SignatureSourceVariableSlot::Local {
            frame,
            simple,
        } = symbol.slot()
        else {
            return false;
        };
        let represented = self.original_variable_formal_occurrences(frame.lexer_config());
        let mut found = false;
        for rows in self.declaration_layouts.values() {
            let Some(rows) = original_declaration_layouts(rows) else {
                continue;
            };
            for row in rows {
                if row.entry.frame() != frame.frame()
                    || row.entry.source() != frame.source()
                    || row.config != frame.lexer_config()
                {
                    continue;
                }
                found = true;
                let Some(topology) = row.entry.original_formal_topology() else {
                    if matches!(
                        row.entry.as_ref(),
                        OriginalDiagnosticFrameEntry::ScriptActivation { .. }
                            | OriginalDiagnosticFrameEntry::LogicalScriptActivation { .. }
                    ) {
                        continue;
                    }
                    return true;
                };
                if topology
                    .parameters()
                    .iter()
                    .any(|parameter| parameter.name.as_slice() == simple.as_bytes())
                    && !represented
                        .iter()
                        .any(|occurrence| occurrence.symbol() == symbol)
                {
                    return true;
                }
            }
        }
        !found
    }

    /// Select the innermost original local naming frame owning a source cursor.
    /// Whitespace belongs to the retained body image; command words and display
    /// scope spans do not manufacture a frame.
    pub(crate) fn original_variable_frame_at_offset(
        &self,
        offset: u32,
        config: tcl_lexer::LexerConfig,
    ) -> Option<SourceOriginalVariableFrame> {
        let origin = self.root_origin.as_ref()?;
        let mut found: Option<SourceOriginalVariableFrame> = None;
        #[cfg(debug_assertions)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_COMPLETION").is_some() {
            eprintln!(
                "ORIGINAL_VARIABLE_FRAME offset={offset} layouts={} deferred={}",
                self.declaration_layouts.len(),
                self.deferred.len(),
            );
        }
        for observations in self.declaration_layouts.values() {
            #[cfg(debug_assertions)]
            if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_COMPLETION")
                .is_some()
            {
                for row in observations {
                    if row.entry.owns_source_cursor(origin, offset) {
                        let layout = match row.entry.frame().layout() {
                            crate::var_resolve::VariableExecutionFrame::Global => "global",
                            crate::var_resolve::VariableExecutionFrame::Procedure { .. } => {
                                "procedure"
                            }
                            crate::var_resolve::VariableExecutionFrame::ReceiverMethod {
                                ..
                            } => "receiver",
                            crate::var_resolve::VariableExecutionFrame::NamespaceActivation {
                                ..
                            } => "namespace-activation",
                            _ => "other",
                        };
                        eprintln!(
                            "ORIGINAL_VARIABLE_FRAME offset={offset} site={} issuer={:?} layout={layout} source_base={} source_bytes={} config={} frame={} origin={} context={}",
                            row.words
                                .first()
                                .map_or(0, |word| word.source().span.start()),
                            row.issuer,
                            row.entry.source().base(),
                            row.entry.source().text.len(),
                            row.config == config,
                            row.snapshot.state.variable_frame == *row.entry.frame(),
                            row.snapshot.state.current_source_origin.as_ref()
                                == Some(&row.entry.source().origin),
                            row.entry
                                .owns_original_context(&row.snapshot.state.source_variables),
                        );
                    }
                }
            }
            let Some(original) = original_declaration_layouts(observations) else {
                continue;
            };
            let first = original.clone().next()?;
            if first.config != config
                || !first.entry.owns_source_cursor(origin, offset)
                || !matches!(
                    first.entry.frame().layout(),
                    crate::var_resolve::VariableExecutionFrame::Procedure { .. }
                        | crate::var_resolve::VariableExecutionFrame::ReceiverMethod { .. }
                        | crate::var_resolve::VariableExecutionFrame::NamespaceActivation { .. }
                )
            {
                continue;
            }
            let frame = SourceOriginalVariableFrame {
                source: first.entry.source().clone(),
                frame: first.entry.frame().clone(),
                config,
                logical_lambda: None,
            };
            match &found {
                Some(previous) if previous.source.text.len() < frame.source.text.len() => {}
                Some(previous) if previous.source.text.len() == frame.source.text.len() => {
                    if previous != &frame {
                        #[cfg(debug_assertions)]
                        if tcl_dialect::trace_env::var_os(
                            "TCL_LSP_TRACE_ORIGINAL_VARIABLE_COMPLETION",
                        )
                        .is_some()
                        {
                            eprintln!(
                                "ORIGINAL_VARIABLE_FRAME offset={offset} stage=conflicting-owners source_bytes={}",
                                frame.source.text.len(),
                            );
                        }
                        return None;
                    }
                }
                _ => found = Some(frame),
            }
        }
        #[cfg(debug_assertions)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_COMPLETION").is_some() {
            eprintln!(
                "ORIGINAL_VARIABLE_FRAME offset={offset} stage=selected found={}",
                found.is_some(),
            );
        }
        found
    }

    /// Select a local naming frame solely from authentic original body rows.
    /// Entered caller alternatives, unknown frames and disagreeing recipes
    /// cannot donate a declaration identity from their displayed scope names.
    pub(crate) fn original_variable_frame_at_span(
        &self,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
    ) -> Option<SourceOriginalVariableFrame> {
        let origin = self.root_origin.as_ref()?;
        let mut found: Option<SourceOriginalVariableFrame> = None;
        for (site, observations) in self.root_layouts_spanning(span.start(), span.end()) {
            if &site.source != origin || site.offset > span.start() {
                continue;
            }
            let Some(original) = original_declaration_layouts(observations) else {
                continue;
            };
            let first = original.clone().next()?;
            if first.config != config
                || !first.entry.owns_source(origin, span.start())
                || !first.words.iter().any(|word| {
                    word.source().span.start() <= span.start()
                        && span.end() <= word.source().span.end()
                })
                || !matches!(
                    first.entry.frame().layout(),
                    crate::var_resolve::VariableExecutionFrame::Procedure { .. }
                        | crate::var_resolve::VariableExecutionFrame::ReceiverMethod { .. }
                        | crate::var_resolve::VariableExecutionFrame::NamespaceActivation { .. }
                )
            {
                continue;
            }
            let frame = SourceOriginalVariableFrame {
                source: first.entry.source().clone(),
                frame: first.entry.frame().clone(),
                config,
                logical_lambda: None,
            };
            match &found {
                Some(previous) if previous.source.text.len() < frame.source.text.len() => {}
                Some(previous) if previous.source.text.len() == frame.source.text.len() => {
                    if previous != &frame {
                        return None;
                    }
                }
                _ => found = Some(frame),
            }
        }
        found
    }
}

pub(super) fn root_diagnostic_namespace(
    state: &ModuleCommandBindings,
    current: &super::SourceNamespaceKey,
) -> Option<super::SourceNamespaceKey> {
    let root = state.source_root_namespace_key()?;
    if state.variable_frame.layout() != &crate::var_resolve::VariableExecutionFrame::Global
        || current != &root
        || state
            .variable_frame
            .namespace_identity()
            .is_some_and(|key| key != &root)
        || (state.baseline.native_entry.is_some()
            && state.variable_frame.namespace_identity() != Some(&root))
    {
        return None;
    }
    Some(root)
}

/// Candidate original source calls retain declaration allocations separately
/// from Registry metadata. This grants native formal layout advice only.
pub(crate) struct OriginalDeclarationCallLayoutAdvice {
    layout: OriginalCompilationLookupAdvice,
}

impl OriginalDeclarationCallLayoutAdvice {
    pub(crate) fn targets(&self) -> &[super::SourceCommandTarget] {
        self.layout.targets()
    }

    pub(crate) fn dialect(&self) -> tcl_registry::InvocationDialect {
        self.layout.dialect()
    }

    /// Original Standalone declaration targets for a conditional source report.
    /// This reborrows the retained row's metadata, never actual point metadata.
    /// Observer applicability, successful lookup and body entry stay separate.
    pub(crate) fn original_authored_source_targets(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<&[super::SourceCommandTarget]> {
        if !self.layout.is_authored_standalone_source() || !self.source_targets_are_closed() {
            return None;
        }
        let metadata = self
            .layout
            .original_declaration_metadata_context(registry, tokens)
            .ok()?;
        if metadata.is_some_and(|metadata| !metadata.is_standalone()) {
            return None;
        }
        Some(self.targets())
    }

    /// Readonly closure of the original source candidates. This never closes
    /// actual handler, operand, provider or activation alternatives.
    pub(crate) fn source_targets_are_closed(&self) -> bool {
        self.layout.closed_lookup() && !self.layout.has_opaque_handler_alternatives()
    }
}

/// An original operand read in an accepted declaration's local frame.
/// This is diagnostic occurrence evidence, not a physical read, contents
/// verdict or represented SSA use. Unknown runtime alternatives remain open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeclarationReadOccurrenceAdvice {
    invocation: CommandAllocationSite,
    name: String,
    source: crate::ir::SourceSite,
    spelling: String,
}

impl DeclarationReadOccurrenceAdvice {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn source(&self) -> &crate::ir::SourceSite {
        &self.source
    }

    pub(crate) fn spelling(&self) -> &str {
        &self.spelling
    }

    /// Existing symbolic value at this diagnostic boundary, conditional on
    /// the original declaration-local name retaining that value. No SSA
    /// definition, use, physical address or executable dependency is added.
    pub(crate) fn diagnostic_version(
        &self,
        ssa: &crate::ssa::SsaFunction,
        block: crate::cfg::BlockId,
        index: usize,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<(crate::ssa::Symbol, crate::ssa::Version)> {
        let tokens = crate::ssa::SsaSourceView::at_statement(ssa, block, index).source_tokens()?;
        let binding = tokens.source_binding.as_ref()?;
        if binding.invocation_site()? != &self.invocation
            || !binding
                .declaration_read_occurrences(registry, tokens)?
                .contains(self)
        {
            return None;
        }
        symbolic_version(ssa, block, index, &self.name)
    }
}

/// A decoded variable operand in the original declaration-local layout.
/// Its symbolic value is conditional advice, never a physical address/read.
pub(crate) struct DeclarationVariableOperandAdvice {
    invocation: CommandAllocationSite,
    word: crate::ir::WordExpr,
    name: String,
}

/// Sealed original source layout for conditional diagnostics. This carrier
/// has no conversion to Native lookup/body/occurrence or execution facts.
pub(crate) struct DeclarationSourceOperandLayoutAdvice {
    pub(crate) config: tcl_lexer::LexerConfig,
    kind: DeclarationSourceOperandLayoutKind,
}

enum DeclarationSourceOperandLayoutKind {
    Original(OriginalCompilationLookupAdvice),
    Configured {
        layout: Arc<super::declaration_preview::OriginalDeclarationSourceOperandLayout>,
        owner: std::sync::Weak<super::OriginalSourceAnalysisOwner>,
        cache: std::sync::Weak<super::declaration_preview::OriginalDeclarationSourceOperandCache>,
    },
}

impl DeclarationSourceOperandLayoutAdvice {
    #[cfg(test)]
    pub(crate) fn dialect(&self) -> tcl_registry::InvocationDialect {
        match &self.kind {
            DeclarationSourceOperandLayoutKind::Original(advice) => advice.dialect(),
            DeclarationSourceOperandLayoutKind::Configured { layout, .. } => layout.dialect(),
        }
    }

    fn current_advice(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<OriginalCompilationLookupAdvice> {
        self.current_source_advice(tokens, false)
    }

    fn current_conditional_source_advice(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<OriginalCompilationLookupAdvice> {
        match &self.kind {
            DeclarationSourceOperandLayoutKind::Configured { layout, .. }
                if !layout.source_operations.is_empty() =>
            {
                self.current_source_advice(tokens, true)
            }
            _ => self.current_advice(tokens),
        }
    }

    fn current_source_advice(
        &self,
        tokens: &crate::ir::CommandTokens,
        conditional_source: bool,
    ) -> Option<OriginalCompilationLookupAdvice> {
        let binding = tokens.source_binding.as_ref()?;
        match &self.kind {
            DeclarationSourceOperandLayoutKind::Original(original) => {
                let current = binding.declaration_operand_layout_advice(tokens)?;
                (current.snapshot == original.snapshot
                    && current.site == original.site
                    && current.words == original.words
                    && current.config == self.config
                    && current.targets == original.targets)
                    .then_some(current)
            }
            DeclarationSourceOperandLayoutKind::Configured {
                layout,
                owner,
                cache,
            } => {
                let owner = owner.upgrade()?;
                let cache = cache.upgrade()?;
                if binding
                    .original_analysis_owner
                    .as_ref()
                    .is_none_or(|actual| !Arc::ptr_eq(actual, &owner))
                    || binding.has_original_source_layout_owner()
                    || binding
                        .original_source_operand_attachment
                        .as_ref()
                        .is_none_or(|attachment| !attachment.owns(&cache, layout.site.offset))
                    || owner.config != self.config
                    || !Arc::ptr_eq(&owner.source, &layout.site.source)
                    || cache
                        .entries
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .get(&layout.site.offset)?
                        .as_ref()
                        .is_none_or(|current| !Arc::ptr_eq(current, layout))
                {
                    return None;
                }
                if conditional_source {
                    layout.conditional_source_advice(tokens)
                } else {
                    layout.metadata_advice(tokens)
                }
            }
        }
    }

    fn current_metadata_context(
        &self,
        registry: &tcl_registry::CommandRegistry,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<crate::registry_invocation::InvocationMetadataSelection<'_>> {
        self.current_conditional_source_advice(tokens)?;
        match &self.kind {
            DeclarationSourceOperandLayoutKind::Original(advice) => advice
                .original_declaration_metadata_context(registry, tokens)
                .ok()
                .map(crate::registry_invocation::InvocationMetadataSelection::from),
            DeclarationSourceOperandLayoutKind::Configured { layout, .. } => layout
                .source_metadata_context(registry, tokens)
                .ok()
                .map(crate::registry_invocation::InvocationMetadataSelection::from),
        }
    }

    /// Registry candidates under the same original source configuration.
    /// The callback receives conditional schema data, never a Normal handler.
    pub(crate) fn with_conditional_registry_facts<T: PartialEq>(
        &self,
        registry: &tcl_registry::CommandRegistry,
        tokens: &crate::ir::CommandTokens,
        mut project: impl FnMut(
            &tcl_registry::InvocationFacts,
            &crate::registry_invocation::EffectiveCommandWords,
            &[crate::registry_invocation::EffectiveInvocationWord],
        ) -> Option<T>,
    ) -> Option<T> {
        let advice = self.current_conditional_source_advice(tokens)?;
        let metadata = self.current_metadata_context(registry, tokens)?.context();
        let mut selected = None;
        for target in advice.targets() {
            if !target.registry_backed {
                return None;
            }
            let effective = crate::registry_invocation::compose_original_effective_words(
                tokens,
                &target.command,
                &target.prepended,
            )?;
            let values = effective
                .words
                .iter()
                .map(|word| {
                    crate::registry_invocation::effective_invocation_word(
                        word,
                        self.config.escapes,
                        tcl_syntax::word_rules::WordValueRules::from_config(&self.config),
                    )
                })
                .collect::<Vec<_>>();
            let words = values
                .iter()
                .map(crate::registry_invocation::EffectiveInvocationWord::as_registry_word)
                .collect::<Vec<_>>();
            let crate::registry_invocation::RegistryInvocationResolution::Resolved(facts) =
                crate::registry_invocation::resolve_registry_words_in_realm_with_metadata_context(
                    registry,
                    metadata,
                    &words,
                    Some(advice.dialect()),
                    advice.realm(),
                )
                .ok()?
            else {
                return None;
            };
            let candidate = project(&facts, &effective, &values)?;
            if selected
                .as_ref()
                .is_some_and(|previous| previous != &candidate)
            {
                return None;
            }
            selected = Some(candidate);
        }
        selected
    }
}

/// One complete written scalar source operand and its symbolic diagnostic use.
/// Folded source bytes acquire no ordinary Native occurrence or cell address.
pub(crate) struct DeclarationSourceVariableOperandAdvice {
    site: CommandAllocationSite,
    word: crate::ir::WordExpr,
    name: String,
    source_operations: Arc<[super::declaration_preview::OriginalDeclarationSourceOperation]>,
}

impl DeclarationSourceVariableOperandAdvice {
    /// Reuse the current original scalar operand receipt without converting
    /// its source spelling into a Native value, cell or invocation fact.
    pub(super) fn matches_original_scalar_read(&self, name: &str) -> bool {
        self.name == name && self.source_operations.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn diagnostic_version(
        &self,
        ssa: &crate::ssa::SsaFunction,
        block: crate::cfg::BlockId,
        index: usize,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<(crate::ssa::Symbol, crate::ssa::Version)> {
        let tokens = crate::ssa::SsaSourceView::at_statement(ssa, block, index).source_tokens()?;
        let current = tokens
            .source_binding
            .as_ref()?
            .declaration_source_variable_operand_advice(registry, tokens, &self.word)?;
        (current.site == self.site
            && current.name == self.name
            && current.source_operations == self.source_operations)
            .then_some(())?;
        symbolic_version(ssa, block, index, &self.name)
    }
}

fn trace_root_script_option<T>(stage: &str, value: Option<T>) -> Option<T> {
    #[cfg(debug_assertions)]
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_TAINT_CU_INGRESS").is_some() {
        eprintln!(
            "TAINT_SCRIPT_INGRESS stage={stage} present={}",
            value.is_some()
        );
    }
    #[cfg(not(debug_assertions))]
    let _ = stage;
    value
}

impl super::SourceInvocationBinding {
    /// Original root-script naming geometry for source analysis only. A held
    /// declaration entry does not issue an entered frame or a selected cell.
    pub(crate) fn original_standalone_script_variable_scope(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<(
        crate::signature_scan::scope::SignatureNamespaceScope,
        tcl_syntax::naming::NamePolicyProtocol,
    )> {
        self.original_standalone_script_scope(tokens, registry, false)
    }

    /// A possible source schema retains original coordinates and all current
    /// lookup obligations; it cannot supply a variable identity or storage.
    pub(crate) fn original_standalone_script_catalogue_candidate(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<super::OriginalCatalogueSourceCandidate> {
        let (namespace, policy) = self.original_standalone_script_scope(tokens, registry, true)?;
        let site = self.invocation_site()?;
        (site.source.source_image().channel() == tcl_lexer::SourceChannel::Document)
            .then_some(())?;
        let config = self.original_lexer_config_for_tokens(tokens)?;
        let original = crate::registry_invocation::original_native_compiler_words(
            site.source.source_image(),
            tokens.words(),
            site.offset,
            config,
        )?;
        let key = crate::signature_scan::scope::SignatureSourceNameKey::from_original_logical_source_word(
            original.first()?, tcl_syntax::word_rules::WordValueRules::from_config(&config), policy,
        )?;
        let head =
            crate::signature_scan::original_name::SourceOriginalNameOccurrence::new(site, key)?;
        self.original_catalogue_source_candidate(tokens, &head, Some(&namespace), registry)
    }

    fn original_standalone_script_scope(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
        catalogue_only: bool,
    ) -> Option<(
        crate::signature_scan::scope::SignatureNamespaceScope,
        tcl_syntax::naming::NamePolicyProtocol,
    )> {
        if tokens.synthetic.is_some() || tokens.source_binding.as_ref() != Some(self) {
            return None;
        }
        let owner = trace_root_script_option("root.owner", self.original_analysis_attachment())?;
        if !owner.metadata.is_standalone() {
            return None;
        }
        let rows = trace_root_script_option(
            "root.layout_rows",
            original_source_catalogue_layouts(trace_root_script_option(
                "root.layout_inventory",
                self.declaration_layout_observations.as_deref(),
            )?),
        )?;
        if rows.clone().any(|row| {
            !matches!(
                row.entry.as_ref(),
                OriginalDiagnosticFrameEntry::RootScript { .. }
            )
        }) {
            return None;
        }
        let config =
            trace_root_script_option("root.config", self.original_lexer_config_for_tokens(tokens))?;
        let retained =
            trace_root_script_option("root.registry", self.retained_registry_for_tokens(tokens))?;
        (retained.snapshot().semantic_key() == registry.snapshot().semantic_key()).then_some(())?;
        let site = trace_root_script_option("root.site", self.invocation_site())?;
        if owner.config != config || !Arc::ptr_eq(&owner.source, &site.source) {
            return None;
        }
        let mut selected = None;
        for row in rows {
            let OriginalDiagnosticFrameEntry::RootScript {
                source,
                frame,
                namespace,
            } = row.entry.as_ref()
            else {
                return None;
            };
            let state = &row.snapshot.state;
            let baseline = &state.baseline;
            if row.config != config
                || row.words.as_ref() != tokens.words()
                || !Arc::ptr_eq(&source.origin, &site.source)
                || source.base() != 0
                || source.text.bytes() != site.source.source_image().bytes()
                || !row.entry.owns_source(&site.source, site.offset)
                || frame.layout() != &crate::var_resolve::VariableExecutionFrame::Global
                || namespace != &row.namespace
                || state.current_source_origin.as_ref() != Some(&site.source)
                || !baseline.metadata_context.is_standalone()
                || baseline.native_entry.is_some()
                || baseline.logical_source_input.is_some()
                || baseline.vendor_source_input.is_some()
                || baseline.hosted_execution_context.is_some()
                || baseline.unknown_entry
                || baseline.registry_owner.as_ref()?.semantic_key()
                    != registry.snapshot().semantic_key()
            {
                return None;
            }
            let policy = trace_root_script_option(
                "root.policy",
                trace_root_script_option("root.execution_policy", baseline.execution_name_policy)?
                    .native_recipe(),
            )?;
            if policy.authority() != tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation {
                return None;
            }
            let scope = trace_root_script_option(
                "root.namespace_geometry",
                if catalogue_only {
                    state
                        .original_command_world
                        .retained_scope_for_source_advice(namespace, policy)
                } else {
                    state.original_namespace_geometry(namespace, policy)
                },
            )?;
            let current = (scope, policy);
            if selected.as_ref().is_some_and(|prior| prior != &current) {
                return None;
            }
            selected = Some(current);
        }
        trace_root_script_option("root.selected", selected)
    }

    pub(crate) fn declaration_source_operand_layout_advice(
        &self,
        registry: &tcl_registry::CommandRegistry,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<DeclarationSourceOperandLayoutAdvice> {
        if tokens.source_binding.as_ref() != Some(self) {
            return None;
        }
        if self.has_original_source_layout_owner() {
            let advice = self.declaration_operand_layout_advice(tokens)?;
            advice
                .original_declaration_metadata_context(registry, tokens)
                .ok()?;
            return Some(DeclarationSourceOperandLayoutAdvice {
                config: advice.config,
                kind: DeclarationSourceOperandLayoutKind::Original(advice),
            });
        }
        let owner = self.original_analysis_owner.as_ref()?;
        let offset = tokens.words().first()?.source().span.start();
        let cache = owner
            .derived_memo()
            .source_operand_layouts
            .get()?
            .upgrade()?;
        let layout = cache
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&offset)?
            .as_ref()?
            .clone();
        let advice = DeclarationSourceOperandLayoutAdvice {
            config: owner.config,
            kind: DeclarationSourceOperandLayoutKind::Configured {
                layout,
                owner: Arc::downgrade(owner),
                cache: Arc::downgrade(&cache),
            },
        };
        advice.current_metadata_context(registry, tokens)?;
        Some(advice)
    }

    pub(crate) fn declaration_source_variable_operand_advice(
        &self,
        registry: &tcl_registry::CommandRegistry,
        tokens: &crate::ir::CommandTokens,
        word: &crate::ir::WordExpr,
    ) -> Option<DeclarationSourceVariableOperandAdvice> {
        let source = self.declaration_source_operand_layout_advice(registry, tokens)?;
        if let DeclarationSourceOperandLayoutKind::Original(_) = &source.kind {
            let original = self.declaration_variable_operand_advice(registry, tokens, word)?;
            return Some(DeclarationSourceVariableOperandAdvice {
                site: original.invocation,
                word: original.word,
                name: original.name,
                source_operations: Arc::from([]),
            });
        }
        if word.source().provenance != crate::ir::Provenance::Source
            || !tokens.words().iter().any(|original| original == word)
        {
            return None;
        }
        let crate::registry_invocation::EffectiveInvocationWord::Literal(name) =
            crate::registry_invocation::effective_invocation_word(
                word,
                source.config.escapes,
                tcl_syntax::word_rules::WordValueRules::from_config(&source.config),
            )
        else {
            return None;
        };
        if tcl_syntax::naming::is_qualified(name.as_bytes())
            || tcl_syntax::naming::split_array_name_braced_for_style(
                &name,
                true,
                source.config.braced_var,
            )
            .1
            .is_some()
        {
            return None;
        }
        let DeclarationSourceOperandLayoutKind::Configured { layout, .. } = &source.kind else {
            return None;
        };
        layout
            .variable_name_is_eligible(registry, &name)
            .then_some(())?;
        Some(DeclarationSourceVariableOperandAdvice {
            site: layout.site.clone(),
            word: word.clone(),
            name,
            source_operations: Arc::clone(&layout.source_operations),
        })
    }
}

impl DeclarationVariableOperandAdvice {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn diagnostic_version(
        &self,
        ssa: &crate::ssa::SsaFunction,
        block: crate::cfg::BlockId,
        index: usize,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<(crate::ssa::Symbol, crate::ssa::Version)> {
        let tokens = crate::ssa::SsaSourceView::at_statement(ssa, block, index).source_tokens()?;
        let binding = tokens.source_binding.as_ref()?;
        if binding.invocation_site()? != &self.invocation
            || binding
                .declaration_variable_operand_advice(registry, tokens, &self.word)?
                .name
                != self.name
        {
            return None;
        }
        symbolic_version(ssa, block, index, &self.name)
    }
}

pub(super) fn symbolic_version(
    ssa: &crate::ssa::SsaFunction,
    block: crate::cfg::BlockId,
    index: usize,
    name: &str,
) -> Option<(crate::ssa::Symbol, crate::ssa::Version)> {
    let symbol = ssa.cell_symbol(name)?;
    let block = ssa.blocks.get(&block)?;
    let version = if index == usize::MAX {
        block.exit_versions.get(&symbol).copied()?
    } else {
        block.statements.get(index)?;
        block.statements[..index]
            .iter()
            .rev()
            .find_map(|statement| statement.defs.get(&symbol).copied())
            .or_else(|| block.entry_versions.get(&symbol).copied())?
    };
    Some((symbol, version))
}

impl SourceCommandBindings {
    /// Called receivers retain a separate declaration preview for source roles.
    /// Only catalogue rows leave the collector; entered observations stay separate.
    pub(super) fn retain_called_receiver_source_catalogues(
        &mut self,
        incoming: &ModuleCommandBindings,
        config: tcl_lexer::LexerConfig,
        registry: &tcl_registry::CommandRegistry,
    ) {
        let mut entries = BTreeMap::new();
        for (body, entry) in self.deferred_entry_states(incoming) {
            if !body.receiver_method
                || body.event.is_some()
                || body.future_frame.is_some()
                || body.statics.is_some()
                || !self
                    .called_implementations
                    .contains(&body.implementation_id())
            {
                continue;
            }
            let key = body.implementation_id();
            match entries.entry(key) {
                std::collections::btree_map::Entry::Vacant(slot) => {
                    slot.insert(Some((body, entry)));
                }
                std::collections::btree_map::Entry::Occupied(mut slot) => {
                    if slot.get().as_ref().is_some_and(|(previous, state)| {
                        previous != &body || !state.same_state(&entry)
                    }) {
                        slot.insert(None);
                    }
                }
            }
        }
        if entries.is_empty() {
            return;
        }
        let mut conditional = self.clone();
        conditional.active_calls.clear();
        conditional.active_variable_observers.clear();
        let mut declarations = std::collections::BTreeSet::new();
        for (body, entry) in entries.into_values().flatten() {
            let Some((declaration, _, _)) = &body.executed_script else {
                continue;
            };
            declarations.insert(declaration.clone());
            conditional.walk_deferred_entry(&body, &entry, config, registry);
        }
        for (site, observations) in &conditional.declaration_layouts {
            let Some(originals) = original_declaration_layouts(observations) else {
                continue;
            };
            for original in originals {
                let OriginalDiagnosticFrameEntry::DeclaredReceiver(body) = original.entry.as_ref()
                else {
                    continue;
                };
                if !declarations.contains(body.declaration()) {
                    continue;
                }
                let mut retained = original.clone();
                retained.issuer = DeclarationLayoutIssuer::ConditionalReceiverSource;
                let rows = self.declaration_layouts.entry(site.clone()).or_default();
                if !rows.contains(&retained) {
                    rows.push(retained);
                }
            }
        }
    }

    pub(super) fn retain_logical_receiver_source_catalogues(
        &mut self,
        config: tcl_lexer::LexerConfig,
        registry: &tcl_registry::CommandRegistry,
    ) {
        let Some(input) = self.original_logical_source_name_advice_input().cloned() else {
            return;
        };
        let Some(origin) = self.source_origin().cloned() else {
            return;
        };
        if !input.has_logical_source_name_context()
            || input.lexer_config() != config
            || input
                .borrowed_context_registry()
                .commands()
                .snapshot()
                .semantic_key()
                != registry.snapshot().semantic_key()
        {
            return;
        }
        let Some(tape) = self.original_source_transition_advice_tape_for(
            input.borrowed_context_registry(),
            super::OriginalSourceCataloguePurpose::CompletePrefix,
        ) else {
            return;
        };
        let Some(commands) = tape.logical_receiver_source_commands(&origin, &input) else {
            return;
        };
        for (body, command) in commands {
            self.retain_one_logical_receiver_source_catalogue(
                body, &command, &origin, &input, config,
            );
        }
    }

    fn retain_one_logical_receiver_source_catalogue(
        &mut self,
        body: Arc<crate::registry_invocation::OriginalSourceDefinitionMemberScriptBody>,
        command: &Arc<crate::registry_invocation::OriginalSourceCommandProjection>,
        origin: &Arc<super::SourceOriginId>,
        input: &crate::analyser::ResolvedAnalysisInput,
        config: tcl_lexer::LexerConfig,
    ) {
        let crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(
            parent,
        ) = body.original_parent_body().source_words().source()
        else {
            return;
        };
        if parent.logical_source_input() != Some(input)
            || !Arc::ptr_eq(&parent.site().source, origin)
            || !parent.matches_source(origin.source_image(), config)
            || !parent.matches_context(input.borrowed_context_registry())
        {
            return;
        }
        // Only the actual retained parent declaration supplies this
        // lineage. A fresh root state cannot stand in for a missing row.
        let Some(parents) = self
            .declaration_layouts
            .get(parent.site())
            .and_then(|rows| original_declaration_layouts(rows))
            .map(|rows| rows.cloned().collect::<Vec<_>>())
        else {
            return;
        };
        if parents.iter().any(|row| {
            row.config != config
                || row.snapshot.state.logical_source_name_advice_input() != Some(input)
                || !row.entry.owns_source(origin, parent.site().offset)
                || !row
                    .entry
                    .owns_original_context(&row.snapshot.state.source_variables)
                || row.snapshot.state.variable_frame != *row.entry.frame()
                || row.snapshot.state.current_source_origin.as_ref() != Some(origin)
                || origin
                    .original_lexical_command(&row.words, parent.site().offset, config)
                    .is_none_or(|original| original.native_words() != parent.original_words())
        }) {
            return;
        }
        let content = body.content_span();
        let Some(bytes) = origin.source_image().bytes().get(content.as_range()) else {
            return;
        };
        // This facet retains unchanged original body geometry. Transformed
        // values require their distinct source correspondence owner.
        if tcl_syntax::word_rules::original_static_word_unicode_value(body.original_container())
            .as_deref()
            != Some(bytes)
        {
            return;
        }
        let Some(source) = super::ExecutedScriptSource::contiguous_image(
            Arc::clone(origin),
            tcl_lexer::SourceImage::from_bytes(bytes, origin.source_image().channel()),
            content.start(),
        ) else {
            return;
        };
        let Some(member) = body.original_words().first() else {
            return;
        };
        let member_offset = member.span().start();
        let entry = Arc::new(OriginalDiagnosticFrameEntry::LogicalReceiver(Arc::new(
            OriginalLogicalReceiverSourceBody {
                body,
                input: input.clone(),
                source,
                frame: crate::var_resolve::VariableExecutionFrame::ReceiverMethod {
                    identity: super::source_activation_name(
                        Some(origin),
                        "logical-receiver-source-layout",
                        member_offset,
                    ),
                },
            },
        )));
        let site = CommandAllocationSite {
            source: Arc::clone(origin),
            offset: command.native_words()[0].span().start(),
        };
        let tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::from_image(origin.source_image()),
            config,
            command.command(),
        );
        let rows = self.declaration_layouts.entry(site).or_default();
        for parent in parents {
            let row = DeclarationLayoutObservation {
                issuer: DeclarationLayoutIssuer::ConditionalReceiverSource,
                entry: Arc::clone(&entry),
                // The held snapshot/namespace belong to the parent
                // declaration. LogicalReceiver cannot project them as a
                // receiver lookup namespace or entered naming context.
                snapshot: parent.snapshot,
                namespace: parent.namespace,
                config,
                words: Arc::from(tokens.words()),
            };
            if !rows.contains(&row) {
                rows.push(row);
            }
        }
    }

    /// Retain original diagnostic layouts after an abrupt source prefix.
    /// The actual abrupt table is borrowed unchanged. No arguments, dispatch,
    /// compiler visits or source effects are evaluated for this suffix.
    pub(super) fn record_unentered_declaration_suffix(
        &mut self,
        image: &tcl_lexer::SourceImage,
        base: u32,
        segments: &[crate::segmenter::SegmentedCommand],
        state: &ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) {
        let snapshot = Arc::new(SourceLookupSnapshot::in_realm(
            state.clone(),
            context.realm(),
        ));
        for segment in segments {
            if segment.is_partial {
                break;
            }
            let tokens = super::source_command_tokens_boxed(image, base, context.config, segment);
            self.record_declaration_operand_layout(state, context, segment.span.start(), &tokens);
            let Some(origin) = state.current_source_origin.as_ref() else {
                break;
            };
            let site = CommandAllocationSite {
                source: Arc::clone(origin),
                offset: segment.span.start(),
            };
            let Some(advice) = super::original_site_operand_layout_advice(
                &site,
                &tokens,
                &snapshot,
                &context.namespace_identity(),
                context.config,
            ) else {
                break;
            };
            let Some(selected) = crate::registry_invocation::declaration_invocation_flow(
                context.registry,
                &tokens,
                &advice,
            ) else {
                break;
            };
            // Without evaluating the command, a changed table or a substituted
            // operand cannot supply a lookup snapshot for its successors.
            if !selected.effects.lookup_stable
                || selected.effects.unknown_writes
                || !matches!(
                    selected.flow,
                    tcl_registry::script_body_flow::ScriptBodyFlow::None
                )
                || selected.effective.words.iter().any(|word| {
                    crate::registry_invocation::effective_invocation_word(
                        word,
                        advice.dialect().lexer_grammar.escapes,
                        advice.dialect().word_values,
                    )
                    .literal_bytes()
                    .is_none()
                })
            {
                break;
            }
        }
    }

    /// Capture before evaluating any operand, using the original declaration
    /// owner or its retained entered-body frame and the current source table.
    /// Earlier replacements remain visible.
    #[inline(never)]
    pub(super) fn record_declaration_operand_layout(
        &mut self,
        state: &ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        offset: u32,
        words: &crate::ir::CommandTokens,
    ) {
        #[cfg(any(test, debug_assertions))]
        let _timing = super::root_owner_work::Timing::at(super::root_owner_work::Query::Layout);
        if state.source_variables.invocation_dialect.is_none() {
            return;
        }
        let Some(origin) = state.current_source_origin.as_ref() else {
            return;
        };
        let (entry, namespace) = if let Some(body) = self.conditional_body_entry_at(origin, offset)
        {
            if !self.original_body_owns_frame(&body, &state.variable_frame) {
                return;
            }
            (
                OriginalDiagnosticFrameEntry::Body(body),
                context.namespace_identity(),
            )
        } else if let Some(body) =
            super::declaration_preview::logical_body_entry::current_entry(state, context, offset)
        {
            (
                OriginalDiagnosticFrameEntry::DeclaredProcedure(body),
                context.namespace_identity(),
            )
        } else if let Some(body) = self.declared_receiver_body_entry_at(origin, offset) {
            if body.preview_frame() != &state.variable_frame {
                return;
            }
            let namespace = body.declaration_namespace_context().clone();
            (
                OriginalDiagnosticFrameEntry::DeclaredReceiver(body),
                namespace,
            )
        } else if let Some(entry) =
            self.original_script_activation_at(state, context, origin, offset)
        {
            (entry, context.namespace_identity())
        } else {
            // A nested parser issues no frame. An immediately selected nested
            // original script (a command substitution, an `expr` operand's
            // `[…]`) still runs in this root frame and namespace, so it keeps
            // the root entry only for its own genuine original vector.
            // Conditional/future bodies and materialised text never qualify.
            if context.source_purpose() != tcl_lexer::SourceSegmentationPurpose::WholeInput
                && (context.body_entry() != super::SourceBodyEntryPurpose::Selected
                    || origin
                        .original_lexical_command(words.words(), offset, context.config)
                        .is_none())
            {
                return;
            }
            let Some(namespace) = root_diagnostic_namespace(state, &context.namespace_identity())
            else {
                return;
            };
            if self.root_origin.as_ref() != Some(origin) {
                return;
            }
            let Some(source) = super::ExecutedScriptSource::contiguous_image(
                Arc::clone(origin),
                origin.source_image().clone(),
                0,
            ) else {
                return;
            };
            (
                OriginalDiagnosticFrameEntry::RootScript {
                    source,
                    frame: state.variable_frame.clone(),
                    namespace: namespace.clone(),
                },
                namespace,
            )
        };
        let entry = Arc::new(entry);
        let observation = DeclarationLayoutObservation {
            issuer: if state.variable_frame == *entry.frame() {
                DeclarationLayoutIssuer::OriginalDeclaration
            } else {
                DeclarationLayoutIssuer::EnteredActivation
            },
            entry,
            snapshot: Arc::new(SourceLookupSnapshot::in_realm(
                state.clone(),
                context.realm(),
            )),
            namespace,
            config: context.config,
            words: Arc::from(words.words()),
        };
        let observations = self
            .declaration_layouts
            .entry(CommandAllocationSite {
                source: Arc::clone(origin),
                offset,
            })
            .or_default();
        if !observations.contains(&observation) {
            observations.push(observation);
        }
    }

    /// Logical source metadata meets all genuine pre-operand occurrences of a
    /// complete head. Runtime opacity is not replaced by a Native lookup.
    pub(crate) fn lexical_source_header_unpositioned(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        name: &str,
    ) -> Option<String> {
        // naming.minifier.logical-source-header
        // docs/design/analysis/name-resolution-proofs/minifier-logical-source-header.md
        if !self.matches_original_source_image(image, config) {
            return None;
        }
        let origin = self.root_origin.as_ref()?;
        let mut selected = None;
        for (site, observations) in &self.declaration_layouts {
            if &site.source != origin {
                continue;
            }
            let rows = original_declaration_layouts(observations)?;
            for row in rows {
                if row.config != config || !row.entry.owns_source(origin, site.offset) {
                    return None;
                }
                let words = crate::registry_invocation::original_native_compiler_words(
                    image,
                    &row.words,
                    site.offset,
                    config,
                )?;
                let values = words
                    .iter()
                    .map(|word| {
                        tcl_syntax::word_rules::original_static_word_ascii_presentation(word)
                    })
                    .collect::<Vec<_>>();
                if values.first()?.as_deref() != Some(name.as_bytes()) {
                    continue;
                }
                let state = &row.snapshot.state;
                if values.iter().any(Option::is_none)
                    || state.baseline.native_entry.is_some()
                    || state.baseline.execution_name_policy.is_some()
                    || state.baseline.hosted_execution_context.is_some()
                    || state.baseline.unknown_entry
                {
                    return None;
                }
                let binding = super::source_binding_projection_in(
                    state,
                    name,
                    &row.namespace,
                    super::CommandTargetLookup::NamedSlots,
                );
                let target = binding.proved_target()?;
                if !target.registry_backed || !target.prepended.is_empty() {
                    return None;
                }
                if selected
                    .as_ref()
                    .is_some_and(|prior| prior != &target.command)
                {
                    return None;
                }
                selected = Some(target.command.clone());
            }
        }
        selected
    }

    pub(super) fn original_logical_source_header_rows(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        words: &[tcl_lexer::NativeWord],
    ) -> Option<(String, Arc<[DeclarationLayoutObservation]>)> {
        let image = words.first()?.image();
        let config = input.lexer_config();
        if self.original_logical_source_name_advice_input() != Some(input)
            || !self.matches_original_source_image(image, config)
        {
            return None;
        }
        let origin = self.root_origin.as_ref()?;
        let offset = words.first()?.tokens().first()?.span.start();
        let site = CommandAllocationSite {
            source: Arc::clone(origin),
            offset,
        };
        let rows = original_declaration_layouts(self.declaration_layouts.get(&site)?)?;
        let head = tcl_syntax::word_rules::original_static_word_ascii_presentation(words.first()?)?;
        let name = std::str::from_utf8(&head).ok()?;
        let mut selected = None;
        let mut retained = Vec::new();
        for row in rows {
            let original = crate::registry_invocation::original_native_compiler_words(
                image, &row.words, offset, config,
            )?;
            let state = &row.snapshot.state;
            if row.config != config
                || !row.entry.owns_source(origin, offset)
                || original.as_slice() != words
                || state.logical_source_name_advice_input() != Some(input)
            {
                return None;
            }
            let binding = super::source_binding_projection_in(
                state,
                name,
                &row.namespace,
                super::CommandTargetLookup::NamedSlots,
            );
            // Retain actual positioned alternatives even when the conditional
            // entry has unenumerated runtime lookup. A known user target,
            // captured prefix or no represented Registry target is terminal.
            if binding.targets.is_empty() {
                return None;
            }
            for target in &binding.targets {
                if !target.registry_backed
                    || !target.prepended.is_empty()
                    || selected
                        .as_ref()
                        .is_some_and(|prior| prior != &target.command)
                {
                    return None;
                }
                selected = Some(target.command.clone());
            }
            retained.push(row.clone());
        }
        Some((selected?, Arc::from(retained)))
    }

    /// Positioned Logical metadata keeps the exact original vector while
    /// retaining unknown operand values. This is the before-argv candidate,
    /// not a proof that argument effects preserve runtime dispatch.
    pub(crate) fn lexical_source_header_for_words(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        words: &[tcl_lexer::NativeWord],
    ) -> Option<String> {
        // naming.minifier.complete-logical-metadata
        // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
        if !self.matches_original_source_image(image, config) {
            return None;
        }
        let origin = self.root_origin.as_ref()?;
        let offset = words.first()?.tokens().first()?.span.start();
        let site = CommandAllocationSite {
            source: Arc::clone(origin),
            offset,
        };
        let rows = original_declaration_layouts(self.declaration_layouts.get(&site)?)?;
        let head = tcl_syntax::word_rules::original_static_word_ascii_presentation(words.first()?)?;
        let name = std::str::from_utf8(&head).ok()?;
        let mut selected = None;
        for row in rows {
            let original = crate::registry_invocation::original_native_compiler_words(
                image, &row.words, offset, config,
            )?;
            let state = &row.snapshot.state;
            if row.config != config
                || !row.entry.owns_source(origin, offset)
                || original.as_slice() != words
                || state.baseline.native_entry.is_some()
                || state.baseline.execution_name_policy.is_some()
                || state.baseline.hosted_execution_context.is_some()
                || state.baseline.unknown_entry
            {
                return None;
            }
            let binding = super::source_binding_projection_in(
                state,
                name,
                &row.namespace,
                super::CommandTargetLookup::NamedSlots,
            );
            let target = binding.proved_target()?;
            if !target.registry_backed
                || !target.prepended.is_empty()
                || selected
                    .as_ref()
                    .is_some_and(|prior| prior != &target.command)
            {
                return None;
            }
            selected = Some(target.command.clone());
        }
        selected
    }

    /// Retain only original declaration observations from a separate preview.
    /// Actual dispatch, reads, effects, coverage and compiler receipts stay in
    /// their independently entered inventories.
    pub(super) fn merge_original_declaration_layouts(&mut self, preview: &Self) {
        for (site, observations) in &preview.declaration_layouts {
            let Some(originals) = original_declaration_layouts(observations) else {
                continue;
            };
            let retained = self.declaration_layouts.entry(site.clone()).or_default();
            for observation in originals {
                if !retained.contains(observation) {
                    retained.push(observation.clone());
                }
            }
        }
    }

    /// Existential declaration layout with unanimous candidate identities.
    /// The original invocation and body recipe must still match. Unknown
    /// runtime alternatives are deliberately not removed by this projection.
    pub(crate) fn declaration_operand_layout_advice(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<OriginalCompilationLookupAdvice> {
        let mut binding = tokens.source_binding.as_ref()?.clone();
        if binding.declaration_layout_observations.is_none() {
            let site = binding.invocation_site()?;
            binding.declaration_layout_observations = self
                .declaration_layouts
                .get(site)
                .map(|observations| Arc::from(observations.as_slice()));
        }
        binding.declaration_operand_layout_advice(tokens)
    }

    /// Original source-backed callees under the recorded pre-operand table.
    /// Unlike Registry metadata advice, this retains a declaration allocation
    /// for the independent native formal binder and supplies no handler facts.
    pub(crate) fn declaration_call_layout_advice(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<OriginalDeclarationCallLayoutAdvice> {
        let mut binding = tokens.source_binding.as_ref()?.clone();
        if binding.declaration_layout_observations.is_none() {
            let site = binding.invocation_site()?;
            binding.declaration_layout_observations = self
                .declaration_layouts
                .get(site)
                .map(|observations| Arc::from(observations.as_slice()));
        }
        Some(OriginalDeclarationCallLayoutAdvice {
            layout: binding.original_declared_layout_advice(tokens, true)?,
        })
    }

    /// Seal only the actual original Logical declaration vector, after its
    /// ordinary token attachments. A reached point keeps its existing dispatch
    /// guards; an absent point retains unknown runtime lookup unchanged.
    pub(super) fn stamp_original_logical_declaration_targets(
        &self,
        tokens: &mut crate::ir::CommandTokens,
    ) {
        // naming.compiler.original-positioned-callee-body-availability
        // docs/design/analysis/name-resolution-proofs/original-positioned-callee-body-availability.md
        let Some(binding) = tokens.source_binding.as_ref() else {
            return;
        };
        let Some(site) = binding.invocation_site() else {
            return;
        };
        if binding.lookup_state.is_some()
            || binding.original_logical_declaration_targets.is_some()
            || self.root_origin.as_ref() != Some(&site.source)
            || self.dispatch_points_at(site.offset).next().is_some()
            || binding != &self.invocation_at_source("", site.offset)
        {
            return;
        }
        let Some(advice) = binding.original_logical_declaration_layout_advice(tokens) else {
            return;
        };
        if !advice.closed_logical_source_lookup() {
            return;
        }
        let receipt = OriginalLogicalDeclarationTargets {
            original_binding: Box::new(binding.clone()),
            targets: advice.targets().to_vec(),
        };
        if let Some(binding) = tokens.source_binding.as_mut() {
            binding.original_logical_declaration_targets = Some(Arc::new(receipt));
        }
    }

    /// Explicit readonly Authored source purpose at an actual retained row.
    /// Raw source tokens and their authentic body/header own this projection;
    /// an existing stronger point/declaration owner is never erased or replaced.
    pub(crate) fn original_authored_declaration_source_tokens(
        &self,
        tokens: &crate::ir::CommandTokens,
        scope: &crate::registry_invocation::OriginalAuthoredSourceBodyScope,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<crate::ir::CommandTokens> {
        if let Some(binding) = tokens.source_binding.as_ref() {
            let original =
                binding.original_authored_source_body_scope_for_name_roles(tokens, registry)?;
            std::ptr::eq(original, scope).then_some(())?;
            let source_only = super::SourceInvocationBinding {
                original_analysis_owner: scope.original_analysis_attachment().cloned(),
                original_authored_source_body_scope: binding
                    .original_authored_source_body_scope
                    .clone(),
                ..Default::default()
            };
            (binding == &source_only).then_some(())?;
        }
        let original = scope.original_words(tokens, registry)?;
        let origin = self.source_origin()?;
        let owner = self.original_analysis_owner.as_ref()?;
        if self.lexer_config != Some(scope.config())
            || origin.source_image() != scope.original_body_word().image()
            || !Arc::ptr_eq(origin, &owner.source)
            || !owner.metadata.is_standalone()
            || self.source_metadata_owner() != &owner.metadata
            || self.final_state.baseline.registry_snapshot.as_ref()
                != Some(&registry.snapshot().semantic_key())
            || scope
                .original_analysis_attachment()
                .is_none_or(|actual| !Arc::ptr_eq(actual, owner))
        {
            return None;
        }
        let site = CommandAllocationSite {
            source: Arc::clone(origin),
            offset: original.first()?.span().start(),
        };
        let observations = self.declaration_layouts.get(&site)?;
        let rows = original_declaration_layouts(observations)?;
        let span = crate::registry_invocation::original_written_script_body_span(
            scope.original_body_word(),
        )?;
        let body_bytes = origin.source_image().bytes().get(span.as_range())?;
        for row in rows {
            let (parent, source, namespace) = match row.entry.as_ref() {
                OriginalDiagnosticFrameEntry::Body(body) => (
                    &body.allocation().site,
                    body.source(),
                    body.namespace_context()?,
                ),
                OriginalDiagnosticFrameEntry::DeclaredProcedure(body) => {
                    (body.declaration_site(), &body.source, &body.namespace)
                }
                _ => return None,
            };
            if parent != scope.original_parent_site()?
                || source.origin != site.source
                || source.base() != span.start()
                || source.text.bytes() != body_bytes
                || namespace != scope.namespace()
                || row.words.as_ref() != tokens.words()
                || row.config != scope.config()
                || !row.entry.owns_source(&site.source, site.offset)
                || !row.snapshot.state.baseline.metadata_context.is_standalone()
            {
                return None;
            }
        }
        let first = observations
            .iter()
            .find(|row| row.issuer == DeclarationLayoutIssuer::OriginalDeclaration)?;
        let binding = super::SourceInvocationBinding {
            unknown: true,
            dispatch_site: Some(site),
            original_analysis_owner: Some(Arc::clone(owner)),
            declaration_layout_observations: Some(Arc::from(observations.as_slice())),
            declaration_flow_inventory: Some(self.current_declaration_flow_inventory()),
            lookup_namespace: first.namespace.advisory_key()?,
            lookup_namespace_key: first.namespace.clone(),
            variable_context: Arc::clone(&first.snapshot.state.source_variables),
            variable_frame: first.snapshot.state.variable_frame.clone(),
            ..Default::default()
        };
        let mut result = tokens.clone();
        result.source_binding = Some(binding);
        Some(result)
    }

    pub(super) fn attach_declaration_operand_layout(
        &self,
        mut binding: super::SourceInvocationBinding,
        origin: Option<&Arc<super::SourceOriginId>>,
        offset: u32,
    ) -> super::SourceInvocationBinding {
        binding.declaration_layout_observations = origin.and_then(|origin| {
            self.declaration_layouts
                .get(&CommandAllocationSite {
                    source: Arc::clone(origin),
                    offset,
                })
                .map(|observations| Arc::from(observations.as_slice()))
        });
        binding.declaration_flow_inventory = binding
            .declaration_layout_observations
            .as_ref()
            .map(|_| self.current_declaration_flow_inventory());
        // Original geometry remains available even when no argv or dispatch
        // was reached. Its issuer is the immutable declaration observation;
        // targets, runtime reachability and compiler authority stay unchanged.
        if binding.dispatch_site.is_none() && binding.declaration_layout_observations.is_some() {
            binding.dispatch_site = origin.map(|source| CommandAllocationSite {
                source: Arc::clone(source),
                offset,
            });
        }
        binding
    }
}

impl super::OriginalCompilationLookupAdvice {
    /// Metadata of this sealed original declaration, including raw preview
    /// vectors which have no reached invocation. Exact source, grammar and
    /// tagged availability remain prerequisites; this grants no dispatch,
    /// Normal handler, Native preparation, entered frame or variable effects.
    /// Only explicit Standalone may return `Ok(None)`.
    pub(crate) fn original_declaration_metadata_context(
        &self,
        registry: &tcl_registry::CommandRegistry,
        tokens: &crate::ir::CommandTokens,
    ) -> Result<
        Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
        crate::registry_invocation::RegistryInvocationDecline,
    > {
        OriginalDeclarationWordSource {
            snapshot: &self.snapshot,
            site: &self.site,
            words: &self.words,
            config: self.config,
            dialect: self.dialect,
        }
        .metadata_context(registry, tokens, OriginalWordSourcePurpose::Invocation)
    }
}

impl super::OriginalCompilationLookupAdvice {
    /// Closed source syntax in the positively retained authored simulation.
    /// The namespace belongs to that source world, independently of a physical
    /// variable frame, Native registration, compilation or successful handler.
    pub(crate) fn closed_authored_simulation_source_lookup(
        &self,
        tokens: &crate::ir::CommandTokens,
        metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
        registry: &tcl_registry::CommandRegistry,
    ) -> bool {
        let Ok(Some(original)) = self.original_declaration_metadata_context(registry, tokens)
        else {
            return false;
        };
        let (Some(held), Some(input)) = (
            original.source_analysis_input(),
            metadata.source_analysis_input(),
        ) else {
            return false;
        };
        let state = &self.snapshot.state;
        held == input
            && !input.has_logical_source_name_context()
            && !input.has_hosted_source_name_context()
            && self.original_source_purpose(metadata, registry)
                == Some(super::OriginalInvocationSourcePurpose::AuthoredSimulation)
            && state.baseline.native_entry.is_none()
            && self
                .source_purpose_state()
                .baseline
                .source_point_entry
                .is_none()
            && state.baseline.logical_source_input.is_none()
            && state.baseline.vendor_source_input.is_none()
            && state.baseline.hosted_execution_context.is_none()
            && !state.baseline.unknown_entry
            && !self.unknown
            && !self.may_be_absent
            && !self.targets.is_empty()
            && !state.has_opaque_domain()
            && !state.source_step_observed()
            && state.command_observers.is_quiet()
            && matches!(&self.namespace, super::SourceNamespaceKey::Authored(_))
            && state.namespaces.contains(&self.namespace)
            && self.targets.iter().all(|target| {
                target.kind == super::BindingKind::Builtin
                    && target.registry_backed
                    && target.prepended.is_empty()
                    && target.implementation_generation == 0
                    && target.runtime_implementation_generation.is_none()
                    && state.retained_target_is_current(target)
                    && !state.source_execution_observed(target.identity.as_ref())
                    && !state.runtime_execution_observed(target.identity.as_ref())
            })
    }

    /// Purpose of this already joined original declaration or compiler layout.
    /// The shared baseline classifier issues no invocation, frame or handler.
    pub(crate) fn original_source_purpose(
        &self,
        metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<super::OriginalInvocationSourcePurpose> {
        self.source_purpose_state()
            .retained_original_source_purpose(metadata, registry)
    }

    /// The state whose baseline supplies this layout's source purpose: its own,
    /// or, when it continues a source past a conditional region, the state of
    /// that source which issued the region.
    fn source_purpose_state(&self) -> &super::ModuleCommandBindings {
        let mut state = &self.snapshot.state;
        while let (Some(point), Some(origin)) = (
            state.baseline.source_point_entry.as_ref(),
            state.current_source_origin.as_deref(),
        ) {
            let Some(issuer) = point.issuing_continuation_state(origin) else {
                break;
            };
            state = issuer;
        }
        state
    }
}

impl super::SourceInvocationBinding {
    /// Keep source-layout purpose separate from actual invocation permission.
    /// A reached point retains its terminal invocation checks; only its absence
    /// permits the existing exact declaration/compiler source correspondence.
    #[doc(hidden)]
    pub fn original_source_advice_purpose(
        &self,
        tokens: &crate::ir::CommandTokens,
        metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<super::OriginalInvocationSourcePurpose> {
        if self.lookup_state.is_some() {
            return self.original_invocation_source_purpose(tokens, metadata, registry);
        }
        if !metadata.matches_registry(registry) {
            return None;
        }
        let advice = self
            .declaration_operand_layout_advice(tokens)
            .or_else(|| self.original_compilation_lookup_advice(tokens))?;
        let original = advice
            .original_declaration_metadata_context(registry, tokens)
            .ok()?;
        match (original, metadata.source_analysis_input()) {
            (Some(original), Some(request)) => {
                let input = original.source_analysis_input()?;
                if input != request && input != &request.for_nested_source() {
                    return None;
                }
            }
            (None, None) if metadata.is_standalone() => {}
            (Some(original), None)
                if original.is_standalone()
                    && metadata.is_standalone()
                    && original.context() == metadata.context() => {}
            _ => return None,
        }
        advice.original_source_purpose(metadata, registry)
    }
}

impl super::SourceCommandBindings {
    /// Purpose of this complete retained source interpretation. Original image,
    /// configuration and metadata remain independent prerequisites; no point,
    /// definition, variable frame or Native permission follows from this tag.
    pub(crate) fn original_source_entry_purpose(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<super::OriginalInvocationSourcePurpose> {
        if !self.matches_original_source_image(image, config)
            || metadata
                .source_analysis_input()
                .is_some_and(|input| input.lexer_config() != config)
        {
            return None;
        }
        self.final_state
            .retained_original_source_purpose(metadata, registry)
    }
}

/// Common immutable declaration-vector join, independent of target selection.
struct OriginalDeclarationWordSource<'a> {
    snapshot: &'a Arc<super::SourceLookupSnapshot>,
    site: &'a CommandAllocationSite,
    words: &'a [crate::ir::WordExpr],
    config: tcl_lexer::LexerConfig,
    dialect: tcl_registry::InvocationDialect,
}

impl<'a> OriginalDeclarationWordSource<'a> {
    fn metadata_context(
        &self,
        registry: &tcl_registry::CommandRegistry,
        tokens: &crate::ir::CommandTokens,
        purpose: OriginalWordSourcePurpose,
    ) -> Result<
        Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
        crate::registry_invocation::RegistryInvocationDecline,
    > {
        use crate::registry_invocation::{
            OwnedInvocationMetadataContext, RegistryInvocationDecline,
        };
        let refusal = RegistryInvocationDecline::IncompleteResolution;
        let snapshot: &'a super::SourceLookupSnapshot = self.snapshot.as_ref();
        let state = &snapshot.state;
        if !purpose.accepts(tokens)
            || tokens.words() != self.words
            || state.current_source_origin.as_ref() != Some(&self.site.source)
            || state.baseline.registry_snapshot.as_ref()
                != Some(&registry.snapshot().semantic_key())
            || state.source_variables.invocation_dialect != Some(self.dialect)
        {
            return Err(refusal);
        }
        self.site
            .source
            .original_lexical_command(tokens.words(), self.site.offset, self.config)
            .ok_or(refusal)?;
        let owner = &state.baseline.metadata_context;
        if let Some(binding) = tokens.source_binding.as_ref() {
            if binding.invocation_site() != Some(self.site)
                || (if purpose == OriginalWordSourcePurpose::ArgumentEvaluation {
                    binding.original_argument_lexer_config(tokens)
                } else {
                    binding.original_lexer_config_for_tokens(tokens)
                }) != Some(self.config)
            {
                return Err(refusal);
            }
            if let Some(point) = &binding.lookup_state {
                if purpose == OriginalWordSourcePurpose::ArgumentEvaluation {
                    return Err(refusal);
                }
                // An explicitly retained invalid reached owner cannot be
                // repaired from an earlier declaration observation.
                binding
                    .original_invocation_metadata_at_point(tokens, registry)
                    .map(|_| ())?;
                if point.state.baseline.metadata_context != *owner {
                    return Err(refusal);
                }
            } else {
                let rows = original_declaration_layouts(
                    binding
                        .declaration_layout_observations
                        .as_deref()
                        .ok_or(refusal)?,
                )
                .ok_or(refusal)?;
                if !rows.clone().all(|row| {
                    row.entry.owns_source(&self.site.source, self.site.offset)
                        && row.words.as_ref() == self.words
                        && row.config == self.config
                        && row.snapshot.state.baseline.metadata_context == *owner
                }) {
                    return Err(refusal);
                }
            }
        }
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
}

impl super::SourceInvocationBinding {
    /// Closed source-only Builtin layout from an explicitly Standalone original
    /// declaration. Reached, Native and supplied source owners cannot borrow it.
    pub(crate) fn original_authored_declaration_scalar_layout(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<super::OriginalCompilationLookupAdvice> {
        if self.lookup_state.is_some()
            || self.has_original_native_source_command()
            || self.has_original_logical_source_advice()
            || tokens.synthetic.is_some()
            || !tokens.original_words_match_argv_text()
        {
            return None;
        }
        let site = self.invocation_site()?;
        let rows = original_declaration_layouts(self.declaration_layout_observations.as_deref()?)?;
        let mut selected = None;
        for row in rows {
            if !row.entry.owns_source(&site.source, site.offset)
                || row.words.as_ref() != tokens.words()
                || !row.snapshot.state.baseline.metadata_context.is_standalone()
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
            let metadata = advice
                .original_declaration_metadata_context(registry, tokens)
                .ok()?;
            if metadata.is_some_and(|metadata| !metadata.is_standalone())
                || !advice.is_authored_standalone_source()
                || advice.has_unclosed_source_coverage()
            {
                return None;
            }
            let [target] = advice.targets() else {
                return None;
            };
            if !target.registry_backed
                || target.kind != super::BindingKind::Builtin
                || !target.prepended.is_empty()
            {
                return None;
            }
            if selected
                .as_ref()
                .is_some_and(|prior: &super::OriginalCompilationLookupAdvice| {
                    prior.targets() != advice.targets() || prior.namespace() != advice.namespace()
                })
            {
                return None;
            }
            selected = Some(advice);
        }
        selected
    }

    /// Complete original argument metadata without selecting a declaration's
    /// handler. Only the genuine argv-evaluation boundary shares this purpose.
    pub(crate) fn original_declaration_argument_metadata(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<crate::registry_invocation::InvocationMetadataContext<'_>> {
        if self.lookup_state.is_some() {
            return None;
        }
        let site = self.invocation_site()?;
        let rows = original_declaration_layouts(self.declaration_layout_observations.as_deref()?)?;
        let mut selected = None;
        for row in rows {
            if !row.entry.owns_source(&site.source, site.offset) {
                return None;
            }
            let source = OriginalDeclarationWordSource {
                snapshot: &row.snapshot,
                site,
                words: &row.words,
                config: row.config,
                dialect: row.snapshot.state.source_variables.invocation_dialect?,
            };
            let metadata = source
                .metadata_context(
                    registry,
                    tokens,
                    OriginalWordSourcePurpose::ArgumentEvaluation,
                )
                .ok()??;
            (row.snapshot
                .state
                .retained_original_source_purpose(metadata, registry)?
                == crate::command_binding::OriginalInvocationSourcePurpose::Logical)
                .then_some(())?;
            if selected.is_some_and(
                |prior: crate::registry_invocation::InvocationMetadataContext<'_>| {
                    prior.source_analysis_input() != metadata.source_analysis_input()
                },
            ) {
                return None;
            }
            selected = Some(metadata);
        }
        selected
    }

    /// Positively Logical metadata from unanimous sealed original declaration
    /// observations. No reached point is manufactured. A present reached owner
    /// is handled by the separate checked point route and cannot fall back here.
    pub(crate) fn original_declaration_metadata_for_source(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<crate::registry_invocation::InvocationMetadataContext<'_>> {
        if self.lookup_state.is_some() {
            return None;
        }
        let site = self.invocation_site()?;
        let rows = original_declaration_layouts(self.declaration_layout_observations.as_deref()?)?;
        let mut selected = None;
        for row in rows {
            if !row.entry.owns_source(&site.source, site.offset)
                || row.words.as_ref() != tokens.words()
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
            let checked = advice
                .original_declaration_metadata_context(registry, tokens)
                .ok()??;
            (row.snapshot
                .state
                .retained_original_source_purpose(checked, registry)?
                == crate::command_binding::OriginalInvocationSourcePurpose::Logical)
                .then_some(())?;
            let input = row
                .snapshot
                .state
                .baseline
                .metadata_context
                .source_analysis_input()?;
            let metadata = row
                .snapshot
                .state
                .baseline
                .metadata_context
                .metadata_context_for_source(registry, row.config, Some(input.unit_profile()))?;
            if selected.is_some_and(
                |prior: crate::registry_invocation::InvocationMetadataContext<'_>| {
                    prior.source_analysis_input() != metadata.source_analysis_input()
                },
            ) {
                return None;
            }
            selected = Some(metadata);
        }
        selected
    }

    /// Original source-backed callees under this binding's unanimous declared
    /// pre-operand table. No entered target or successful call is supplied.
    pub(crate) fn declaration_call_layout_advice(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<OriginalDeclarationCallLayoutAdvice> {
        Some(OriginalDeclarationCallLayoutAdvice {
            layout: self.original_declared_layout_advice(tokens, true)?,
        })
    }

    /// Conditional diagnostic cell keys selected by exact original static
    /// operands at their own original point. Alias bindings and byte names
    /// come from every retained snapshot, never from reporting spellings or
    /// the caller's current table. This grants no Must destruction or Normal.
    pub(crate) fn declaration_variable_argument_keys(
        &self,
        tokens: &crate::ir::CommandTokens,
        arguments: &[usize],
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Vec<crate::var_resolve::VariableCellKey>> {
        self.declaration_operand_layout_advice(tokens)?;
        let rows = original_declaration_layouts(self.declaration_layout_observations.as_deref()?)?;
        let mut unanimous = None;
        for row in rows {
            let state = &row.snapshot.state;
            let context = &state.source_variables;
            let policy = context.execution_name_policy?.native_recipe()?;
            let mut keys = Vec::new();
            for &argument in arguments {
                let word = tokens.words().get(argument.checked_add(1)?)?;
                let input = crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(
                    self.original_source_name_key_at_span(
                        word.source().span,
                        row.config,
                        context.invocation_dialect?.word_values,
                        policy,
                    )?,
                );
                let selected = policy.recipe().combined_variable_input(input.bytes());
                if !original_local_primary_is_byte_unique(state, policy, selected.root().selected())
                {
                    return None;
                }
                let place = crate::var_resolve::resolve_original_name_input(
                    &input,
                    context,
                    registry,
                    false,
                    tcl_registry::TraceOperation::Read,
                );
                keys.push(crate::var_resolve::canonical_binding_value_key(&place)?);
            }
            if unanimous.as_ref().is_some_and(|previous| previous != &keys) {
                return None;
            }
            unanimous = Some(keys);
        }
        unanimous
    }

    /// Original direct operand reads whose source and declaration-local scope
    /// agree across every retained observation. Aliases, formal inputs,
    /// qualified names and unavailable geometry provide no local advice.
    pub(crate) fn declaration_read_occurrences(
        &self,
        registry: &tcl_registry::CommandRegistry,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<Vec<DeclarationReadOccurrenceAdvice>> {
        let layout = self.declaration_operand_layout_advice(tokens)?;
        let site = self.invocation_site()?;
        let observations =
            original_declaration_layouts(self.declaration_layout_observations.as_deref()?)?;
        let end = tokens.words().last()?.source().span.end();
        let config = layout.config;
        let mut reads = Vec::new();
        for access in &tokens.variable_accesses {
            if !direct_declaration_operand_owner(&access.owner, site)
                || access.source.span.start() < site.offset
                || access.source.span.end() > end
            {
                continue;
            }
            let bytes = access.original_spelling.as_bytes();
            let start = usize::try_from(access.source.span.start()).ok()?;
            if bytes.first() != Some(&b'$')
                || site
                    .source
                    .source_image()
                    .bytes()
                    .get(start..start.checked_add(bytes.len())?)
                    != Some(bytes)
            {
                continue;
            }
            let Some(reference) = tcl_lexer::word_parts::scan_var_ref(bytes, 0, config)
                .ok()
                .flatten()
                .filter(|reference| reference.next == bytes.len())
            else {
                continue;
            };
            let Ok(name) = std::str::from_utf8(reference.name) else {
                continue;
            };
            let contexts = access
                .context_alternatives()
                .iter()
                .filter(|context| {
                    observations
                        .clone()
                        .all(|observation| observation.entry.owns_original_context(context))
                })
                .collect::<Vec<_>>();
            if tcl_syntax::naming::is_qualified(reference.name)
                || contexts.is_empty()
                || contexts
                    .iter()
                    .any(|context| local_read_scope_is_excluded(context, name))
                || observations.clone().any(|observation| {
                    let context = &observation.snapshot.state.source_variables;
                    !observation
                        .entry
                        .owns_source(&site.source, access.source.span.start())
                        || observation
                            .entry
                            .parameters()
                            .iter()
                            .any(|formal| formal.name == name)
                        || contexts
                            .iter()
                            .any(|read| context.activation != read.activation)
                        || local_read_scope_is_excluded(context, name)
                        || observation
                            .source_body_name_ownership(
                                registry,
                                crate::script_binds::Ownership::ScopeAliases,
                            )
                            .is_none_or(|effects| {
                                effects.opaque
                                    || effects.names.iter().any(|candidate| candidate == name)
                            })
                })
            {
                continue;
            }
            let read = DeclarationReadOccurrenceAdvice {
                invocation: site.clone(),
                name: name.to_owned(),
                source: access.source.clone(),
                spelling: access.original_spelling.clone(),
            };
            if !reads.contains(&read) {
                reads.push(read);
            }
        }
        Some(reads)
    }

    /// Lexical local-frame classification of the accepted declaration. This
    /// is only a body-layout condition; it supplies no installed alias or
    /// runtime frame and cannot replace the normal variable context.
    pub(crate) fn declaration_alias_frame_advice(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<tcl_registry::VariableAliasFrame> {
        self.declaration_operand_layout_advice(tokens)?;
        let observations =
            original_declaration_layouts(self.declaration_layout_observations.as_deref()?)?;
        if observations.clone().any(|observation| {
            !matches!(
                observation.entry.frame().layout(),
                crate::var_resolve::VariableExecutionFrame::Procedure { .. }
                    | crate::var_resolve::VariableExecutionFrame::ReceiverMethod { .. }
            )
        }) {
            return None;
        }
        Some(tcl_registry::VariableAliasFrame::Procedure)
    }

    /// A written, statically decoded scalar operand in the accepted local
    /// declaration frame. Alias/formal/trace or conflicting layouts decline.
    /// Callers separately select the operation/operand role; this is no access.
    pub(crate) fn declaration_variable_operand_advice(
        &self,
        registry: &tcl_registry::CommandRegistry,
        tokens: &crate::ir::CommandTokens,
        word: &crate::ir::WordExpr,
    ) -> Option<DeclarationVariableOperandAdvice> {
        let layout = self.declaration_operand_layout_advice(tokens)?;
        let site = self.invocation_site()?;
        if word.source().provenance != crate::ir::Provenance::Source
            || !tokens.words().iter().any(|original| original == word)
        {
            return None;
        }
        // Original overrides belong to this layout, independently of its dialect.
        let config = layout.config;
        let crate::registry_invocation::EffectiveInvocationWord::Literal(name) =
            crate::registry_invocation::effective_invocation_word(
                word,
                config.escapes,
                tcl_syntax::word_rules::WordValueRules::from_config(&config),
            )
        else {
            return None;
        };
        if tcl_syntax::naming::is_qualified(name.as_bytes())
            || tcl_syntax::naming::split_array_name_braced_for_style(&name, true, config.braced_var)
                .1
                .is_some()
        {
            return None;
        }
        let observations =
            original_declaration_layouts(self.declaration_layout_observations.as_deref()?)?;
        if observations.clone().any(|observation| {
            let context = &observation.snapshot.state.source_variables;
            observation
                .entry
                .parameters()
                .iter()
                .any(|formal| formal.name == name)
                || local_read_scope_is_excluded(context, &name)
                || observation
                    .source_body_name_ownership(
                        registry,
                        crate::script_binds::Ownership::ScopeAliasesAndTraceRegistrations,
                    )
                    .is_none_or(|effects| effects.opaque || effects.names.contains(&name))
        }) {
            return None;
        }
        Some(DeclarationVariableOperandAdvice {
            invocation: site.clone(),
            word: word.clone(),
            name,
        })
    }

    /// Reborrow the actual unanimous original body/declaration row for source
    /// parameter reports. Missing allocated-body inventory cannot mint a Body
    /// from an uninstalled declaration or replace a present reached owner.
    pub(crate) fn original_body_parameter_source_scope(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<OriginalBodyParameterSourceScope> {
        let rows = original_declaration_layouts(self.declaration_layout_observations.as_deref()?)?;
        let first = rows.clone().next()?;
        if !matches!(
            first.entry.as_ref(),
            OriginalDiagnosticFrameEntry::Body(_)
                | OriginalDiagnosticFrameEntry::DeclaredProcedure(_)
        ) || first.entry.original_formal_topology().is_none()
        {
            return None;
        }
        let layout = self.original_unanimous_declared_layout_advice(
            tokens,
            original_body_parameter_source_layout,
        )?;
        Some(OriginalBodyParameterSourceScope {
            entry: Arc::clone(&first.entry),
            layout,
        })
    }

    /// Candidate layout retained independently of actual normal handler facts.
    /// Exact declaration owner, original geometry and every observation agree.
    pub(crate) fn declaration_operand_layout_advice(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<OriginalCompilationLookupAdvice> {
        self.original_declared_layout_advice(tokens, false)
    }

    pub(super) fn original_declared_layout_advice(
        &self,
        tokens: &crate::ir::CommandTokens,
        source_calls: bool,
    ) -> Option<OriginalCompilationLookupAdvice> {
        self.original_unanimous_declared_layout_advice(
            tokens,
            if source_calls {
                super::original_declaration_call_layout_advice
            } else {
                super::original_operand_layout_advice
            },
        )
    }

    /// Closed conditional callees in a positively retained Logical source
    /// world. Original vectors, input, namespace and every declaration row
    /// must agree. A source procedure target is not a physical allocation;
    /// callers separately join its original declaration/body inventory.
    pub(crate) fn original_logical_declaration_call_targets(
        &self,
        tokens: &crate::ir::CommandTokens,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Vec<super::SourceCommandTarget>> {
        let metadata = self.original_invocation_metadata_for_input(tokens, input, registry)?;
        (self.original_invocation_source_purpose(tokens, metadata, registry)?
            == crate::command_binding::OriginalInvocationSourcePurpose::Logical)
            .then_some(())?;
        let advice = self.original_logical_declaration_layout_advice(tokens)?;
        advice.closed_logical_source_lookup().then_some(())?;
        // A reached invocation retains its post-operand dispatch guards. A
        // declaration-only source uses its independently sealed conditional
        // targets, while every raw binding and original row must stay unchanged.
        if self.lookup_state.is_some() {
            if self.unknown || self.may_be_absent || self.targets.as_slice() != advice.targets() {
                return None;
            }
        } else if !self
            .original_logical_declaration_targets
            .as_ref()
            .is_some_and(|receipt| receipt.matches(self, &advice))
        {
            return None;
        }
        let mut targets = advice.targets().to_vec();
        for target in &mut targets {
            target.implementation_allocation = None;
        }
        Some(targets)
    }

    /// Complete Logical declaration layout in its original source world.
    /// Internal source allocations serve lexical correspondence only; callers
    /// cannot use this purpose for Native formal storage or actual activation.
    pub(super) fn original_logical_declaration_layout_advice(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<OriginalCompilationLookupAdvice> {
        self.original_unanimous_declared_layout_advice(
            tokens,
            super::original_logical_declaration_call_layout_advice,
        )
    }

    fn original_unanimous_declared_layout_advice(
        &self,
        tokens: &crate::ir::CommandTokens,
        project: fn(
            &super::SourceInvocationBinding,
            &crate::ir::CommandTokens,
            &Arc<super::SourceLookupSnapshot>,
            &super::SourceNamespaceKey,
            tcl_lexer::LexerConfig,
        ) -> Option<OriginalCompilationLookupAdvice>,
    ) -> Option<OriginalCompilationLookupAdvice> {
        let binding = self;
        let site = super::trace_rbs_operand_option("declaration.site", binding.invocation_site())?;
        let observations = super::trace_rbs_operand_option(
            "declaration.original_rows",
            original_declaration_layouts(super::trace_rbs_operand_option(
                "declaration.rows",
                self.declaration_layout_observations.as_deref(),
            )?),
        )?;
        let mut unanimous: Option<OriginalCompilationLookupAdvice> = None;
        for observation in observations {
            // Entered activations retain their own immutable points; they do
            // not participate in this original declaration's unanimity.
            if !observation.entry.owns_source(&site.source, site.offset) {
                return super::trace_rbs_operand_option("declaration.row_owner", None);
            }
            let advice = super::trace_rbs_operand_option(
                "declaration.row_project",
                project(
                    binding,
                    tokens,
                    &observation.snapshot,
                    &observation.namespace,
                    observation.config,
                ),
            )?;
            if unanimous.as_ref().is_some_and(|previous| {
                previous.targets != advice.targets
                    || previous.dialect != advice.dialect
                    || previous.realm() != advice.realm()
                    || previous.alias_frame() != advice.alias_frame()
                    || previous.frame() != advice.frame()
                    || previous.namespace() != advice.namespace()
                    || previous.unknown != advice.unknown
                    || previous.may_be_absent != advice.may_be_absent
            }) {
                return super::trace_rbs_operand_option("declaration.unanimity", None);
            }
            unanimous = Some(advice);
        }
        super::trace_rbs_operand_option("declaration.result", unanimous)
    }
}

fn original_body_parameter_source_layout(
    binding: &super::SourceInvocationBinding,
    tokens: &crate::ir::CommandTokens,
    snapshot: &Arc<SourceLookupSnapshot>,
    namespace: &super::SourceNamespaceKey,
    config: tcl_lexer::LexerConfig,
) -> Option<OriginalCompilationLookupAdvice> {
    let rows = original_declaration_layouts(binding.declaration_layout_observations.as_deref()?)?;
    let entry = &rows.clone().next()?.entry;
    let layout =
        super::original_operand_layout_advice(binding, tokens, snapshot, namespace, config)?;
    layout
        .closed_body_parameter_source_entry_lookup(entry)
        .then_some(layout)
}

pub(super) fn local_read_scope_is_excluded(
    context: &crate::var_resolve::ResolveContext,
    name: &str,
) -> bool {
    context.globals.contains(name)
        || context.ns_vars.contains(name)
        || context.upvar_aliases.contains_key(name)
        || context.instance_vars.contains(name)
        || context.traced.contains(name)
        || !context.untracked_traces.is_empty()
}

pub(super) fn direct_declaration_operand_owner(
    owner: &super::SourceVariableEvaluationOwner,
    site: &CommandAllocationSite,
) -> bool {
    let mut pending = vec![owner];
    while let Some(owner) = pending.pop() {
        match owner {
            super::SourceVariableEvaluationOwner::InvocationArguments { invocation, .. }
                if invocation == site => {}
            super::SourceVariableEvaluationOwner::Alternatives(owners) if !owners.is_empty() => {
                pending.extend(owners);
            }
            _ => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_nested_root_activation_purpose_tests;

#[cfg(test)]
#[path = "logical_purpose_tests.rs"]
mod logical_purpose_tests;

#[cfg(test)]
mod original_logical_declaration_receipt_tests;

#[cfg(test)]
mod authored_source_parent_tests;
