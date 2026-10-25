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
            Self::RootScript { namespace, .. } | Self::ScriptActivation { namespace, .. } => {
                Some(namespace)
            }
        };
        SourceCommandBindings::context_owns_frame(context, self.frame(), namespace)
    }

    pub(super) fn source(&self) -> &super::ExecutedScriptSource {
        match self {
            Self::Body(body) => body.source(),
            Self::DeclaredReceiver(body) => body.source(),
            Self::LogicalReceiver(body) => &body.source,
            Self::DeclaredProcedure(body) => &body.source,
            Self::RootScript { source, .. } | Self::ScriptActivation { source, .. } => source,
        }
    }

    pub(super) fn frame(&self) -> &crate::var_resolve::VariableExecutionFrame {
        match self {
            Self::Body(body) => body.frame(),
            Self::DeclaredReceiver(body) => body.preview_frame(),
            Self::LogicalReceiver(body) => &body.frame,
            Self::DeclaredProcedure(body) => &body.frame,
            Self::RootScript { frame, .. } | Self::ScriptActivation { frame, .. } => frame,
        }
    }

    pub(super) fn parameters(&self) -> &[tcl_syntax::formal_params::FormalParameter] {
        match self {
            Self::Body(body) => body.parameters(),
            Self::DeclaredReceiver(body) => body.parameters(),
            Self::DeclaredProcedure(body) => &body.parameters,
            Self::LogicalReceiver(_) | Self::RootScript { .. } | Self::ScriptActivation { .. } => {
                &[]
            }
        }
    }

    pub(super) fn original_formal_topology(
        &self,
    ) -> Option<&super::formal_topology::OriginalFormalTopology> {
        match self {
            Self::Body(body) => body.original_formal_topology(),
            Self::DeclaredReceiver(body) => body.original_formal_topology(),
            Self::DeclaredProcedure(body) => body.original_parameters.as_ref(),
            Self::LogicalReceiver(_) | Self::RootScript { .. } | Self::ScriptActivation { .. } => {
                None
            }
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
    let matched = match alias.target().slot() {
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
    };
    if !matched {
        return false;
    }
    true
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeclarationLayoutObservation {
    pub(super) issuer: DeclarationLayoutIssuer,
    pub(super) entry: Arc<OriginalDiagnosticFrameEntry>,
    pub(super) snapshot: Arc<SourceLookupSnapshot>,
    pub(super) namespace: super::SourceNamespaceKey,
    pub(super) config: tcl_lexer::LexerConfig,
    pub(super) words: Arc<[crate::ir::WordExpr]>,
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
    let first = original.clone().next()?;
    if original.clone().any(|observation| {
        observation.entry != first.entry
            || observation.config != first.config
            || observation.words != first.words
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
        return None;
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
        let entry = Arc::new(OriginalDiagnosticFrameEntry::ScriptActivation {
            source,
            frame: state.variable_frame.clone(),
            namespace,
            config: context.config,
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
            } if frame == &state.variable_frame && frame == context.frame && selected == &namespace && *config == context.config)
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
        for (site, rows) in &self.declaration_layouts {
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
        for observations in self.declaration_layouts.values() {
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
                        return None;
                    }
                }
                _ => found = Some(frame),
            }
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
        for (site, observations) in &self.declaration_layouts {
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
        let owner = self.original_analysis_attachment()?;
        if !owner.metadata.is_standalone() {
            return None;
        }
        let rows =
            original_source_catalogue_layouts(self.declaration_layout_observations.as_deref()?)?;
        if rows.clone().any(|row| {
            !matches!(
                row.entry.as_ref(),
                OriginalDiagnosticFrameEntry::RootScript { .. }
            )
        }) {
            return None;
        }
        let config = self.original_lexer_config_for_tokens(tokens)?;
        let retained = self.retained_registry_for_tokens(tokens)?;
        (retained.snapshot().semantic_key() == registry.snapshot().semantic_key()).then_some(())?;
        let site = self.invocation_site()?;
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
            let policy = baseline.execution_name_policy?.native_recipe()?;
            if policy.authority() != tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation {
                return None;
            }
            let scope = if catalogue_only {
                state
                    .original_command_world
                    .retained_scope_for_source_advice(namespace, policy)
            } else {
                state.original_namespace_geometry(namespace, policy)
            }?;
            let current = (scope, policy);
            if selected.as_ref().is_some_and(|prior| prior != &current) {
                return None;
            }
            selected = Some(current);
        }
        selected
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
            && state.baseline.source_point_entry.is_none()
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
        self.snapshot
            .state
            .retained_original_source_purpose(metadata, registry)
    }
}

impl super::SourceInvocationBinding {
    /// Keep source-layout purpose separate from actual invocation permission.
    /// A reached point retains its terminal invocation checks; only its absence
    /// permits the existing exact declaration/compiler source correspondence.
    pub(crate) fn original_source_advice_purpose(
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
        let site = binding.invocation_site()?;
        let observations =
            original_declaration_layouts(self.declaration_layout_observations.as_deref()?)?;
        let mut unanimous: Option<OriginalCompilationLookupAdvice> = None;
        for observation in observations {
            // Entered activations retain their own immutable points; they do
            // not participate in this original declaration's unanimity.
            if !observation.entry.owns_source(&site.source, site.offset) {
                return None;
            }
            let advice = project(
                binding,
                tokens,
                &observation.snapshot,
                &observation.namespace,
                observation.config,
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
                return None;
            }
            unanimous = Some(advice);
        }
        unanimous
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
mod tests {
    #[test]
    fn original_parameter_source_scope_keeps_allocated_and_declared_owners_separate() {
        fn check(
            bindings: &super::SourceCommandBindings,
            registry: &tcl_registry::CommandRegistry,
            source: &str,
            config: tcl_lexer::LexerConfig,
            expected_allocated: Option<bool>,
        ) {
            let offset = u32::try_from(source.find("upvar").unwrap()).unwrap();
            let segment = crate::segmenter::segment_commands_with_offset_and_config(
                "upvar 1 $name cell",
                offset,
                config,
            )
            .remove(0);
            let mut tokens = crate::ir::CommandTokens::from_segmented(
                &tcl_lexer::SourceMap::new(source),
                config,
                &segment,
            );
            bindings.stamp_original_tokens(&mut tokens);
            let binding = tokens.source_binding.as_ref().unwrap();
            let scope = binding
                .original_body_parameter_source_scope(&tokens)
                .expect("actual source row and its own body/formal owner");
            let allocated = matches!(
                scope.entry.as_ref(),
                super::OriginalDiagnosticFrameEntry::Body(_)
            );
            let declared = matches!(
                scope.entry.as_ref(),
                super::OriginalDiagnosticFrameEntry::DeclaredProcedure(_)
            );
            assert!(allocated || declared);
            if let Some(expected) = expected_allocated {
                assert_eq!(allocated, expected);
            }
            assert!(scope.matches_parameters(&["name", "spare"]));
            assert!(!scope.matches_parameters(&["spare", "name"]));
            assert_eq!(
                scope
                    .original_words(&tokens, config, registry)
                    .unwrap()
                    .len(),
                4
            );
            if declared {
                assert!(
                    bindings
                        .conditional_body_entry_at(bindings.source_origin().unwrap(), offset)
                        .is_none()
                );
            } else {
                assert!(
                    bindings
                        .conditional_body_entry_at(bindings.source_origin().unwrap(), offset)
                        .is_some()
                );
                let second_offset = u32::try_from(source.rfind("upvar").unwrap()).unwrap();
                assert_ne!(second_offset, offset);
                let second = bindings
                    .conditional_body_entry_at(bindings.source_origin().unwrap(), second_offset)
                    .unwrap();
                assert!(
                    scope
                        .layout
                        .original_body_parameter_source_words(&tokens, &second, config, registry,)
                        .is_none()
                );
            }
            let mut config = config;
            config.strict_quoting = !config.strict_quoting;
            assert!(scope.original_words(&tokens, config, registry).is_none());
            let mut withdrawn = binding.clone();
            withdrawn.declaration_layout_observations = None;
            assert!(
                withdrawn
                    .original_body_parameter_source_scope(&tokens)
                    .is_none()
            );
        }
        let source = "proc p {name spare} {upvar 1 $name cell; set cell VALUE}\nproc q {name spare} {upvar 1 $name cell; set cell VALUE}";
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let config = tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
        let bindings = super::SourceCommandBindings::analyse(source, config, registry);
        check(&bindings, registry, source, config, Some(true));
        let analysis = crate::analyser::Analyser::new().analyse(source, "jimtcl");
        let input = analysis.resolved_input.as_ref().unwrap();
        check(
            analysis
                .retained_command_realm()
                .unwrap()
                .source_bindings_ref(),
            input.borrowed_context_registry().commands(),
            source,
            input.lexer_config(),
            None,
        );
    }

    #[test]
    fn logical_receiver_catalogues_retain_parent_member_and_vector_without_native_context() {
        let source = "oo::class create C {method m {arg} {set local $arg}}";
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
        let input = analysis.resolved_input.as_ref().unwrap();
        let bindings = analysis
            .retained_command_realm()
            .unwrap()
            .source_bindings_ref();
        let image = tcl_lexer::SourceImage::document(source);
        let offset = u32::try_from(source.find("set local").unwrap()).unwrap();
        let cursor = offset + 5;
        let binding = bindings.invocation_at_source("", offset);
        let rows = binding
            .declaration_layout_observations
            .as_deref()
            .expect("genuine retained Logical receiver catalogue rows");
        let original = super::original_source_catalogue_layouts(rows)
            .unwrap()
            .next()
            .unwrap();
        let super::OriginalDiagnosticFrameEntry::LogicalReceiver(body) = original.entry.as_ref()
        else {
            panic!("distinct Logical member-body issuer");
        };
        assert_eq!(
            original.issuer,
            super::DeclarationLayoutIssuer::ConditionalReceiverSource
        );
        assert_eq!(body.input, *input);
        assert_eq!(body.body.original_words()[0].try_text().unwrap(), "method");
        assert_eq!(body.source.try_text().unwrap(), "set local $arg");
        assert!(
            !original
                .entry
                .owns_original_context(&original.snapshot.state.source_variables)
        );
        assert!(!original.entry.owns_frame(bindings, original.entry.frame()));
        assert!(original.entry.original_formal_topology().is_none());
        assert!(super::original_declaration_layouts(rows).is_none());
        assert!(binding.original_recorded_command().is_none());
        let frame = bindings
            .logical_source_member_frame_at_offset(input, &image, cursor, input.lexer_config())
            .expect("actual source-only receiver frame");
        assert!(matches!(
            frame.frame().layout(),
            crate::var_resolve::VariableExecutionFrame::ReceiverMethod { .. }
        ));
        assert_eq!(frame.source().try_text().unwrap(), "set local $arg");
        assert!(
            bindings
                .original_variable_frame_at_offset(cursor, input.lexer_config())
                .is_none()
        );
        let crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(
            parent,
        ) = body.body.original_parent_body().source_words().source()
        else {
            panic!("actual source parent declaration");
        };
        let mut missing = bindings.clone();
        assert!(missing.declaration_layouts.remove(parent.site()).is_some());
        assert!(
            missing
                .logical_source_member_frame_at_offset(input, &image, cursor, input.lexer_config(),)
                .is_none()
        );
        let mut changed_config = input.lexer_config();
        changed_config.strict_quoting = !changed_config.strict_quoting;
        assert!(
            bindings
                .logical_source_member_frame_at_offset(input, &image, cursor, changed_config,)
                .is_none()
        );
        let changed_image = tcl_lexer::SourceImage::document(&source.replace("local", "other"));
        assert!(
            bindings
                .logical_source_member_frame_at_offset(
                    input,
                    &changed_image,
                    cursor,
                    input.lexer_config(),
                )
                .is_none()
        );
    }

    fn original_tokens(
        source: &str,
        body: &str,
        offset: usize,
    ) -> (super::SourceCommandBindings, crate::ir::CommandTokens) {
        original_tokens_in("tcl8.6", source, body, offset)
    }

    fn original_tokens_in(
        profile: &str,
        source: &str,
        body: &str,
        offset: usize,
    ) -> (super::SourceCommandBindings, crate::ir::CommandTokens) {
        original_tokens_in_image(
            profile,
            &tcl_lexer::SourceImage::document(source),
            body,
            offset,
        )
    }

    fn original_tokens_in_image(
        profile: &str,
        image: &tcl_lexer::SourceImage,
        body: &str,
        offset: usize,
    ) -> (super::SourceCommandBindings, crate::ir::CommandTokens) {
        let dialect = tcl_registry::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::of_dialect_name(Some(profile)).unwrap(),
        );
        original_tokens_in_image_with_config(
            profile,
            image,
            body,
            offset,
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
        )
    }

    fn original_tokens_in_image_with_config(
        profile: &str,
        image: &tcl_lexer::SourceImage,
        body: &str,
        offset: usize,
        config: tcl_lexer::LexerConfig,
    ) -> (super::SourceCommandBindings, crate::ir::CommandTokens) {
        let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
        let dialect = tcl_registry::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::of_dialect_name(Some(profile)).unwrap(),
        );
        let bindings = super::SourceCommandBindings::analyse_image_in_frame_with_options(
            image,
            &crate::var_resolve::VariableExecutionFrame::Global,
            config,
            registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(dialect),
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..super::super::SourceAnalysisOptions::default()
            },
        )
        .unwrap();
        let segment = crate::segmenter::segment_commands_with_offset_and_config(
            body,
            u32::try_from(offset).unwrap(),
            config,
        )
        .remove(0);
        let mut tokens =
            crate::ir::CommandTokens::from_segmented(&image.source_map(), config, &segment);
        bindings.stamp_original_tokens(&mut tokens);
        (bindings, tokens)
    }

    fn issuer_guard_summary(
        observations: &[super::DeclarationLayoutObservation],
    ) -> Vec<[bool; 7]> {
        let original = observations.iter().find(|observation| {
            observation.issuer == super::DeclarationLayoutIssuer::OriginalDeclaration
        });
        observations
            .iter()
            .map(|observation| {
                [
                    observation.issuer == super::DeclarationLayoutIssuer::OriginalDeclaration,
                    original.is_some_and(|first| observation.entry == first.entry),
                    original.is_some_and(|first| observation.config == first.config),
                    original.is_some_and(|first| observation.words == first.words),
                    observation.snapshot.state.variable_frame == *observation.entry.frame(),
                    observation.snapshot.state.current_source_origin.as_ref()
                        == Some(&observation.entry.source().origin),
                    observation
                        .entry
                        .owns_original_context(&observation.snapshot.state.source_variables),
                ]
            })
            .collect()
    }

    #[test]
    fn original_variable_cursor_at_body_end_keeps_occurrence_membership_half_open() {
        // Implementation contract: naming.variable.original-body-cursor-frame

        let source = concat!(
            "set shared 1\n",
            "proc first {argument} {set local 1; global shared; upvar #0 shared link; puts $}\n",
            "proc second {argument} {set foreign 1; puts $}\n",
        );
        let mut analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let image = tcl_lexer::SourceImage::document(source);
        let config = analysis.body_lexer_config.unwrap();
        analysis.global_scope.variables.clear();
        analysis.all_procs.clear();
        let first_end = u32::try_from(source.find("puts $").unwrap() + "puts $".len()).unwrap();
        let second_end = u32::try_from(source.rfind("puts $").unwrap() + "puts $".len()).unwrap();
        let frame = |offset| analysis.original_variable_frame_in_source(&image, config, offset);
        let first = frame(first_end).expect("the cursor precedes the first authentic body closer");
        let second =
            frame(second_end).expect("the cursor precedes the second authentic body closer");
        assert_eq!(frame(first_end - 1), Some(first.clone()));
        assert_eq!(frame(second_end - 1), Some(second.clone()));
        assert_ne!(first, second);
        assert_eq!(frame(first_end + 1), None);
        assert_eq!(frame(second_end + 1), None);
        let bindings = analysis
            .retained_command_realm()
            .unwrap()
            .source_bindings_ref();
        let origin = bindings.root_origin.as_ref().unwrap();
        let entry = bindings
            .declaration_layouts
            .values()
            .find_map(|rows| {
                super::original_declaration_layouts(rows)?.find_map(|row| {
                    (row.entry.frame() == first.frame() && row.entry.source() == first.source())
                        .then(|| row.entry.clone())
                })
            })
            .expect("the frame has its independent original declaration entry");
        assert!(entry.owns_source(origin, first_end - 1));
        assert!(!entry.owns_source(origin, first_end));
        assert!(entry.owns_source_cursor(origin, first_end));
        assert!(!entry.owns_source_cursor(origin, first_end + 1));
        assert!(
            analysis
                .original_variable_frame_in_source(
                    &tcl_lexer::SourceImage::document(&format!("{source} ")),
                    config,
                    first_end,
                )
                .is_none()
        );
        assert!(
            analysis
                .original_variable_frame_in_source(
                    &image,
                    tcl_lexer::LexerConfig {
                        expand_syntax: !config.expand_syntax,
                        ..config
                    },
                    first_end,
                )
                .is_none()
        );
    }

    #[test]
    fn declaration_layout_issuer_preserves_two_actual_caller_activations() {
        let body = "expr {$n + 1}";
        let source = "proc p {n} {expr {$n + 1}}; p 1; p 2";
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let (bindings, tokens) =
                original_tokens_in(dialect, source, body, source.find("expr").unwrap());
            let binding = tokens.source_binding.as_ref().unwrap();
            let unchanged = binding.clone();
            let observations = binding.declaration_layout_observations.as_deref().unwrap();
            let originals =
                super::original_declaration_layouts(observations).unwrap_or_else(|| {
                    panic!(
                        "{dialect}: original issuer/entry/config/words/frame/source/context: {:?}",
                        issuer_guard_summary(observations)
                    )
                });
            let original = originals.clone().next().unwrap();
            let entered = observations
                .iter()
                .filter(|observation| {
                    observation.issuer == super::DeclarationLayoutIssuer::EnteredActivation
                })
                .map(|observation| observation.snapshot.state.variable_frame.clone())
                .collect::<std::collections::HashSet<_>>();
            assert!(entered.len() >= 2, "{dialect}: distinct actual calls");
            assert!(
                originals
                    .clone()
                    .all(|observation| observation.entry == original.entry)
            );
            let advice = binding
                .declaration_operand_layout_advice(&tokens)
                .expect("original layout survives actual-frame joins");
            assert_eq!(advice.frame(), original.entry.frame(), "{dialect}");
            assert!(advice.closed_lookup(), "{dialect}");
            let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
            assert!(
                binding.declaration_flow_report(registry).is_some(),
                "{dialect}"
            );
            assert_eq!(binding, &unchanged, "advice never edits actual receipts");
            let site = binding.invocation_site().unwrap();
            assert!(bindings.declaration_layouts[site].len() > entered.len());
        }
    }

    #[test]
    fn declaration_layout_issuer_requires_original_source_and_unanimity() {
        use std::sync::Arc;
        let source = "proc p {n} {expr {$n + 1}}; p 1; p 2";
        let (_, tokens) = original_tokens(source, "expr {$n + 1}", source.find("expr").unwrap());
        let binding = tokens.source_binding.as_ref().unwrap();
        let observations = binding.declaration_layout_observations.as_deref().unwrap();
        let original = super::original_declaration_layouts(observations)
            .unwrap_or_else(|| {
                panic!(
                    "original issuer/entry/config/words/frame/source/context: {:?}",
                    issuer_guard_summary(observations)
                )
            })
            .next()
            .unwrap();
        let entered = observations
            .iter()
            .find(|observation| {
                observation.issuer == super::DeclarationLayoutIssuer::EnteredActivation
            })
            .unwrap();
        let with_observations = |observations| {
            let mut changed = binding.clone();
            changed.declaration_layout_observations = Some(Arc::from(observations));
            changed
        };
        let missing = with_observations(vec![entered.clone()]);
        assert!(missing.declaration_operand_layout_advice(&tokens).is_none());
        let mut wrong_frame = entered.clone();
        wrong_frame.issuer = super::DeclarationLayoutIssuer::OriginalDeclaration;
        let wrong = with_observations(vec![original.clone(), wrong_frame]);
        assert!(wrong.declaration_operand_layout_advice(&tokens).is_none());

        let mut unknown = original.clone();
        let mut state = unknown.snapshot.state.clone();
        state.mark_opaque_binding_mutation();
        unknown.snapshot = Arc::new(super::SourceLookupSnapshot::in_realm(
            state,
            unknown.snapshot.realm,
        ));
        let ambiguous = with_observations(vec![original.clone(), unknown]);
        assert!(
            ambiguous
                .declaration_operand_layout_advice(&tokens)
                .is_none()
        );

        // Equal authored images intentionally share semantic source identity.
        // A different original input channel supplies genuinely foreign
        // provenance even when its bytes and command geometry are identical.
        let (_, foreign_tokens) = original_tokens_in_image(
            "tcl8.6",
            &tcl_lexer::SourceImage::native(source.as_bytes()),
            "expr {$n + 1}",
            source.find("expr").unwrap(),
        );
        let foreign = super::original_declaration_layouts(
            foreign_tokens
                .source_binding
                .as_ref()
                .unwrap()
                .declaration_layout_observations
                .as_deref()
                .unwrap(),
        )
        .unwrap()
        .next()
        .unwrap();
        let foreign = with_observations(vec![original.clone(), foreign.clone()]);
        assert!(foreign.declaration_operand_layout_advice(&tokens).is_none());

        let mut changed_tokens = tokens.clone();
        changed_tokens.word_exprs[1] = crate::ir::WordExpr::BracedLiteral {
            text: "$n + 99".into(),
            source: tokens.words()[1].source().clone(),
        };
        assert!(
            binding
                .declaration_operand_layout_advice(&changed_tokens)
                .is_none()
        );
    }

    #[test]
    fn repeated_declarations_keep_original_compilers_separate_from_body_allocations() {
        let body = "expr {$n + 1}";
        let source = "foreach generation {A B} {proc p {n} {expr {$n + 1}}}; p 1; p 2";
        let (_, tokens) = original_tokens(source, body, source.find(body).unwrap());
        let binding = tokens.source_binding.as_ref().unwrap();
        let advice = binding.declaration_operand_layout_advice(&tokens).expect(
            "same original lexical declaration can retain operand geometry across repetitions",
        );
        assert!(!advice.targets().is_empty());
        let original = binding
            .admitted_inline_invocation()
            .expect("unchanged original expression compiler registration");
        assert!(original.target.registry_backed);
        assert!(original.target.implementation_allocation.is_none());
        assert_eq!(
            original.operation,
            tcl_registry::SemanticOperationId::StructuredLowering(
                tcl_registry::hooks::LoweringHookId::Expr,
            )
        );
        assert_eq!(
            binding.native_compilation_admission_selection(),
            tcl_registry::native_compilation::NativeCompilationSelection::Inline {
                operation: original.operation,
                guard: tcl_registry::native_compilation::NativeCompilationGuard::BeforeArguments,
            }
        );
        let mut changed = tokens.clone();
        changed.word_exprs.pop();
        assert!(
            binding
                .declaration_operand_layout_advice(&changed)
                .is_none()
        );
    }

    #[test]
    fn replaced_declaration_handler_withdraws_original_layout_advice() {
        let body = "expr {$n + 1}";
        let source = "proc expr args {return CUSTOM}; proc p {n} {expr {$n + 1}}; p 1; p 2";
        let (_, tokens) = original_tokens(source, body, source.find(body).unwrap());
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(binding.declaration_operand_layout_advice(&tokens).is_none());
        let replacement = binding
            .proved_execution_target()
            .expect("known replacement procedure");
        assert!(!replacement.registry_backed);
        assert!(replacement.matches_authored_implementation(source, 0));
        assert_eq!(
            binding.native_compilation_admission_selection(),
            tcl_registry::native_compilation::NativeCompilationSelection::Generic
        );
    }

    #[test]
    fn original_lookup_completeness_keeps_unknown_absent_and_namespace_alternatives() {
        use crate::var_resolve::VariableExecutionFrame;
        let source = "proc p {} {expr {3.5}}";
        let (_, tokens) = original_tokens(source, "expr {3.5}", source.find("expr").unwrap());
        let binding = tokens.source_binding.as_ref().unwrap();
        let mut advice = binding.declaration_operand_layout_advice(&tokens).unwrap();
        assert!(advice.closed_lookup());
        advice.unknown = true;
        assert!(!advice.closed_lookup());
        assert!(
            !advice.targets().is_empty(),
            "May candidates remain available"
        );
        advice.unknown = false;
        advice.may_be_absent = true;
        assert!(!advice.closed_lookup());
        advice.may_be_absent = false;
        let original = advice.snapshot.clone();
        for frame in [
            VariableExecutionFrame::ReceiverMethod {
                identity: "receiver-preview".into(),
            },
            VariableExecutionFrame::Selected {
                selector: tcl_registry::FrameLevel::Relative(1),
                namespace: None,
            },
            VariableExecutionFrame::Unknown,
        ] {
            let mut state = original.state.clone();
            state.variable_frame = frame;
            advice.snapshot = std::sync::Arc::new(super::super::SourceLookupSnapshot::in_realm(
                state,
                original.realm,
            ));
            assert!(
                !advice.closed_lookup(),
                "declaration namespace cannot supply an unknown activation namespace"
            );
            assert!(!advice.targets().is_empty());
        }
        let mut state = original.state.clone();
        state.variable_frame = VariableExecutionFrame::NamespaceIdentity {
            namespace: super::super::SourceNamespaceKey::Authored("::other".into()),
            frame: Box::new(state.variable_frame),
        };
        advice.snapshot = std::sync::Arc::new(super::super::SourceLookupSnapshot::in_realm(
            state,
            original.realm,
        ));
        assert!(
            !advice.closed_lookup(),
            "foreign typed namespace cannot complete the original lookup"
        );
        advice.snapshot = original;
        assert!(advice.closed_lookup());
    }

    #[test]
    fn declaration_variable_operands_preserve_decoded_name_and_decline_aliases() {
        // naming.compiler.original-analysis-metadata-context

        // Alias exclusion requires complete original body ownership, not an
        // installed alias, entered variable frame or successful read.
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        for (source, body, expected) in [
            (
                "proc p {} {set l {a b c}; lset l $j $v}",
                "lset l $j $v",
                Some("l"),
            ),
            ("proc p {} {incr {$n}}", "incr {$n}", Some("$n")),
            ("proc p {} {global l; lset l $j $v}", "lset l $j $v", None),
            ("proc p {l} {lset l $j $v}", "lset l $j $v", None),
            ("proc p {} {lset a(k) $j $v}", "lset a(k) $j $v", None),
        ] {
            let (_, tokens) = original_tokens(source, body, source.find(body).unwrap());
            let binding = tokens.source_binding.as_ref().unwrap();
            let operand =
                binding.declaration_variable_operand_advice(registry, &tokens, &tokens.words()[1]);
            assert_eq!(
                operand
                    .as_ref()
                    .map(super::DeclarationVariableOperandAdvice::name),
                expected,
                "{source}"
            );
            let mut changed = tokens.words()[1].clone();
            match &mut changed {
                crate::ir::WordExpr::Literal { text, .. }
                | crate::ir::WordExpr::BracedLiteral { text, .. } => *text = "different".into(),
                _ => panic!("fixed literal operand"),
            }
            assert!(
                binding
                    .declaration_variable_operand_advice(registry, &tokens, &changed)
                    .is_none()
            );
        }
    }

    #[test]
    fn declaration_variable_operands_use_the_original_selected_word_configuration() {
        let name = concat!("local", "\\", "\n", "name");
        let source = format!("proc p {{}} {{incr {{{name}}}}}");
        let body = format!("incr {{{name}}}");
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let dialect = tcl_registry::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::of_dialect_name(Some("tcl8.6")).unwrap(),
        );
        for brace in [
            tcl_dialect::BraceBackslashNewline::Folds,
            tcl_dialect::BraceBackslashNewline::Literal,
        ] {
            let mut config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
            config.brace_backslash_newline = brace;
            config.strict_quoting = true;
            let (bindings, tokens) = original_tokens_in_image_with_config(
                "tcl8.6",
                &tcl_lexer::SourceImage::document(&source),
                &body,
                source.find(&body).unwrap(),
                config,
            );
            let binding = tokens.source_binding.as_ref().unwrap();
            let layout = binding
                .declaration_source_operand_layout_advice(registry, &tokens)
                .unwrap();
            let original_layout = binding.declaration_operand_layout_advice(&tokens);
            let original_operand =
                binding.declaration_variable_operand_advice(registry, &tokens, &tokens.words()[1]);
            match brace {
                tcl_dialect::BraceBackslashNewline::Folds => {
                    assert!(original_layout.is_none());
                    assert!(original_operand.is_none());
                }
                tcl_dialect::BraceBackslashNewline::Literal => {
                    assert!(original_layout.is_some());
                    assert_eq!(
                        original_operand
                            .as_ref()
                            .map(super::DeclarationVariableOperandAdvice::name),
                        Some(name)
                    );
                }
            }
            assert_eq!(layout.config, config);
            assert_eq!(layout.dialect().word_values.brace, brace);
            let operand = binding
                .declaration_source_variable_operand_advice(registry, &tokens, &tokens.words()[1])
                .expect("genuine original scalar operand under its selected configuration");
            let expected = match brace {
                tcl_dialect::BraceBackslashNewline::Folds => "local name",
                tcl_dialect::BraceBackslashNewline::Literal => name,
            };
            assert_eq!(operand.name(), expected, "{brace:?}");
            assert!(
                crate::registry_invocation::declaration_increment_advice(registry, &tokens)
                    .is_some()
            );
            let mut changed = tokens.clone();
            changed.argv_texts[1] = "other".into();
            assert!(
                binding
                    .declaration_source_variable_operand_advice(
                        registry,
                        &changed,
                        &tokens.words()[1]
                    )
                    .is_none()
            );
            drop(bindings);
        }
    }

    #[test]
    fn declaration_mapped_source_operands_keep_full_vector_owner_and_conflict_refusals() {
        let name = concat!("local", "\\", "\n", "name");
        let source = format!("proc mapped {{}} {{incr {{{name}}}}}");
        let body = format!("incr {{{name}}}");
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let config = tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
        let (bindings, tokens) = original_tokens_in_image_with_config(
            "tcl8.6",
            &tcl_lexer::SourceImage::document(&source),
            &body,
            source.find(&body).unwrap(),
            config,
        );
        let binding = tokens.source_binding.as_ref().unwrap();
        let advice = binding
            .declaration_source_operand_layout_advice(registry, &tokens)
            .expect("actual mapped original body source vector");
        assert!(advice.current_advice(&tokens).is_some());
        assert!(advice.current_metadata_context(registry, &tokens).is_some());
        assert!(
            advice
                .current_advice(&tokens)
                .unwrap()
                .original_declaration_metadata_context(registry, &tokens)
                .is_err(),
            "the old Native declaration facade still requires its own invocation owner"
        );
        assert!(binding.declaration_operand_layout_advice(&tokens).is_none());

        let mut withdrawn = bindings.clone();
        withdrawn.declaration_layouts.clear();
        withdrawn.retain_original_declaration_source_operand_layouts(registry);
        let mut restamped = tokens.clone();
        withdrawn.stamp_original_tokens(&mut restamped);
        assert!(advice.current_advice(&restamped).is_none());
        assert!(
            restamped
                .source_binding
                .as_ref()
                .unwrap()
                .declaration_source_operand_layout_advice(registry, &restamped)
                .is_none()
        );
        // Joining source alternatives must retain the attachment only when
        // both operands have the same actual original inventory receipt.
        for (left, right) in [(&tokens, &restamped), (&restamped, &tokens)] {
            let mut joined = left.clone();
            joined
                .source_binding
                .as_mut()
                .unwrap()
                .join(right.source_binding.as_ref().unwrap());
            assert!(advice.current_advice(&joined).is_none());
            assert!(
                joined
                    .source_binding
                    .as_ref()
                    .unwrap()
                    .declaration_source_operand_layout_advice(registry, &joined)
                    .is_none()
            );
        }
        let mut equal_join = tokens.clone();
        equal_join.source_binding.as_mut().unwrap().join(binding);
        assert!(advice.current_advice(&equal_join).is_some());
        // The retained original tokens still describe their actual old immutable
        // source analysis, independently of the modified clone's inventory.
        assert!(advice.current_advice(&tokens).is_some());

        let mut changed_world = bindings.clone();
        std::sync::Arc::make_mut(&mut changed_world.final_state).mark_opaque_binding_mutation();
        let mut changed_world_tokens = tokens.clone();
        changed_world.stamp_original_tokens(&mut changed_world_tokens);
        assert!(advice.current_advice(&changed_world_tokens).is_none());

        let mut lost_owner = tokens.clone();
        lost_owner
            .source_binding
            .as_mut()
            .unwrap()
            .original_analysis_owner = None;
        assert!(advice.current_advice(&lost_owner).is_none());
        let mut lost_word = tokens.clone();
        lost_word.argv_texts[1] = "other".into();
        assert!(advice.current_advice(&lost_word).is_none());
        let foreign = tcl_registry::model::ingress::static_context_for("jimtcl").commands();
        assert!(
            binding
                .declaration_source_operand_layout_advice(foreign, &tokens)
                .is_none()
        );

        // A genuine new analysis of identical source and a different complete
        // configuration cannot donate its binding to this retained receipt.
        let mut literal = config;
        literal.brace_backslash_newline = tcl_dialect::BraceBackslashNewline::Literal;
        literal.strict_quoting = !config.strict_quoting;
        let (_other_owner, other_tokens) = original_tokens_in_image_with_config(
            "tcl8.6",
            &tcl_lexer::SourceImage::document(&source),
            &body,
            source.find(&body).unwrap(),
            literal,
        );
        assert!(advice.current_advice(&other_tokens).is_none());

        let super::DeclarationSourceOperandLayoutKind::Configured { layout, cache, .. } =
            &advice.kind
        else {
            panic!("distinct source-only mapped carrier");
        };
        cache
            .upgrade()
            .unwrap()
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(layout.site.offset, None);
        assert!(advice.current_advice(&tokens).is_none());
        assert!(
            binding
                .declaration_source_operand_layout_advice(registry, &tokens)
                .is_none()
        );
        drop(bindings);
    }

    #[test]
    fn declaration_configured_scalar_operands_keep_existing_receiver_and_scope_refusals() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut config = tcl_lexer::LexerConfig::from_grammar(
            tcl_dialect::DialectProfile::find("tcl8.6").unwrap().grammar,
        );
        config.brace_backslash_newline = tcl_dialect::BraceBackslashNewline::Literal;
        for (source, body, expected) in [
            ("proc p {} {incr {$n}}", "incr {$n}", Some("$n")),
            ("proc p {} {incr {π}}", "incr {π}", Some("π")),
            ("proc p {} {incr {arr(key)}}", "incr {arr(key)}", None),
            (
                r"proc p {} {incr local\x3a\x3aname}",
                r"incr local\x3a\x3aname",
                None,
            ),
            ("proc p {l} {incr l}", "incr l", None),
            ("proc p {} {global l; incr l}", "incr l", None),
            ("proc p {} {upvar #0 original l; incr l}", "incr l", None),
            (
                "proc p {} {trace add variable l write callback; incr l}",
                "incr l",
                None,
            ),
        ] {
            let (_, tokens) = original_tokens_in_image_with_config(
                "tcl8.6",
                &tcl_lexer::SourceImage::document(source),
                body,
                source.find(body).unwrap(),
                config,
            );
            let operand = tokens
                .source_binding
                .as_ref()
                .unwrap()
                .declaration_variable_operand_advice(registry, &tokens, &tokens.words()[1]);
            assert_eq!(
                operand
                    .as_ref()
                    .map(super::DeclarationVariableOperandAdvice::name),
                expected,
                "{source}"
            );
        }
    }

    fn assert_scalar_original_layout_kind(
        brace: tcl_dialect::BraceBackslashNewline,
        source_layout: &super::DeclarationSourceOperandLayoutAdvice,
        original_operand: Option<&super::DeclarationVariableOperandAdvice>,
        expected: Option<&str>,
        source: &str,
    ) {
        match brace {
            tcl_dialect::BraceBackslashNewline::Folds => {
                assert!(matches!(
                    source_layout.kind,
                    super::DeclarationSourceOperandLayoutKind::Configured { .. }
                ));
                assert!(original_operand.is_none());
            }
            tcl_dialect::BraceBackslashNewline::Literal => {
                // No body transform occurred: its genuine original
                // owner keeps the ordinary trace-name veto, just as in
                // the unchanged-body controls above.
                assert!(matches!(
                    source_layout.kind,
                    super::DeclarationSourceOperandLayoutKind::Original(_)
                ));
                assert_eq!(
                    original_operand.map(super::DeclarationVariableOperandAdvice::name),
                    expected,
                    "unchanged original body: {source}"
                );
            }
        }
    }

    #[test]
    fn declaration_scalar_trace_exclusions_retain_other_names_and_mapped_source_configurations() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        for (source, expected) in [
            (
                "proc trace_local {} {trace add variable l write callback; incr l}",
                None,
            ),
            (
                "proc trace_other {} {trace add variable other write callback; incr l}",
                Some("l"),
            ),
            (
                "proc trace_dynamic {} {trace add variable $target write callback; incr l}",
                None,
            ),
        ] {
            let (_owner, tokens) =
                original_tokens(source, "incr l", source.rfind("incr l").unwrap());
            let advice = tokens
                .source_binding
                .as_ref()
                .unwrap()
                .declaration_variable_operand_advice(registry, &tokens, &tokens.words()[1]);
            assert_eq!(
                advice
                    .as_ref()
                    .map(super::DeclarationVariableOperandAdvice::name),
                expected,
                "{source}"
            );
        }
        let name = concat!("local", "\\", "\n", "name");
        for same_name in [false, true] {
            let target = if same_name {
                format!("{{{name}}}")
            } else {
                "other".to_owned()
            };
            let body = format!("trace add variable {target} write callback; incr {{{name}}}");
            let source = format!("proc trace_mapped {{}} {{{body}}}");
            let command = format!("incr {{{name}}}");
            for brace in [
                tcl_dialect::BraceBackslashNewline::Folds,
                tcl_dialect::BraceBackslashNewline::Literal,
            ] {
                let mut config =
                    tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
                config.brace_backslash_newline = brace;
                config.strict_quoting = true;
                let (_owner, tokens) = original_tokens_in_image_with_config(
                    "tcl8.6",
                    &tcl_lexer::SourceImage::document(&source),
                    &command,
                    source.rfind(&command).unwrap(),
                    config,
                );
                let binding = tokens.source_binding.as_ref().unwrap();
                let advice = binding.declaration_source_variable_operand_advice(
                    registry,
                    &tokens,
                    &tokens.words()[1],
                );
                let expected = if same_name {
                    None
                } else {
                    Some(match brace {
                        tcl_dialect::BraceBackslashNewline::Folds => "local name",
                        tcl_dialect::BraceBackslashNewline::Literal => name,
                    })
                };
                assert_eq!(
                    advice
                        .as_ref()
                        .map(super::DeclarationSourceVariableOperandAdvice::name),
                    expected,
                    "{brace:?}: {source}"
                );
                let source_layout = binding
                    .declaration_source_operand_layout_advice(registry, &tokens)
                    .expect("actual selected source owner");
                let original_operand = binding.declaration_variable_operand_advice(
                    registry,
                    &tokens,
                    &tokens.words()[1],
                );
                assert_scalar_original_layout_kind(
                    brace,
                    &source_layout,
                    original_operand.as_ref(),
                    expected,
                    &source,
                );
            }
        }
    }

    #[test]
    fn mapped_source_continuation_keeps_original_prefix_and_observer_debt() {
        let name = concat!("local", "\\", "\n", "name");
        let body = format!("trace add variable other write callback; incr {{{name}}}");
        let source = format!("proc source_prefix {{}} {{{body}}}");
        let command = format!("incr {{{name}}}");
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let config = tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
        let (_owner, tokens) = original_tokens_in_image_with_config(
            "tcl8.6",
            &tcl_lexer::SourceImage::document(&source),
            &command,
            source.rfind(&command).unwrap(),
            config,
        );
        let binding = tokens.source_binding.as_ref().unwrap();
        let advice = binding
            .declaration_source_operand_layout_advice(registry, &tokens)
            .expect("actual conditional source continuation");
        assert!(
            advice.current_advice(&tokens).is_none(),
            "no unchanged Native lookup world follows a result registration"
        );
        assert!(advice.current_conditional_source_advice(&tokens).is_some());
        let super::DeclarationSourceOperandLayoutKind::Configured { layout, .. } = &advice.kind
        else {
            panic!("mapped readonly source purpose");
        };
        assert_eq!(layout.source_operations.len(), 1);
        let operation = &layout.source_operations[0].operation;
        assert!(operation.obligations().contains(
            &tcl_registry::source_operation::SourceOperationObligation::ObserverApplicability
        ));
        assert!(
            operation
                .effects()
                .accesses()
                .iter()
                .any(|access| access.mode != tcl_registry::EffectAccessMode::Read)
        );
        let operand = binding
            .declaration_source_variable_operand_advice(registry, &tokens, &tokens.words()[1])
            .unwrap();
        assert_eq!(operand.name(), "local name");
        assert_eq!(operand.source_operations, layout.source_operations);
        assert!(
            binding
                .declaration_variable_operand_advice(registry, &tokens, &tokens.words()[1])
                .is_none()
        );
    }

    #[test]
    fn original_destruction_operand_keys_follow_the_retained_alias_point() {
        // Implementation contract: naming.variable.original-destruction-operand-keys

        let source = "proc f {} {set original 1; upvar 0 original linked; unset linked}";
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
            let (_, tokens) = original_tokens_in(
                dialect,
                source,
                "unset linked",
                source.find("unset").unwrap(),
            );
            let binding = tokens.source_binding.as_ref().unwrap();
            let keys = binding
                .declaration_variable_argument_keys(&tokens, &[0], registry)
                .unwrap_or_else(|| panic!("{dialect}: original destruction operand"));
            let [crate::var_resolve::VariableCellKey::Activation { simple, .. }] = keys.as_slice()
            else {
                panic!("{dialect}: independently owned activation key");
            };
            assert_eq!(
                simple.as_bytes(),
                b"original",
                "{dialect}: followed original alias, not reporting linked"
            );
            assert!(
                binding
                    .declaration_variable_argument_keys(&tokens, &[1], registry)
                    .is_none()
            );
        }
    }

    #[test]
    fn declaration_read_occurrences_keep_missing_reads_without_physical_ssa() {
        // naming.compiler.original-analysis-metadata-context

        // Alias exclusion requires complete original body ownership, not an
        // installed alias, entered variable frame or successful read.
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let source = "proc p {} {namespace upvar ::ns a alias; return $missing}";
        let (_, tokens) =
            original_tokens(source, "return $missing", source.find("return").unwrap());
        let reads = tokens
            .source_binding
            .as_ref()
            .unwrap()
            .declaration_read_occurrences(registry, &tokens)
            .expect("original declaration read inventory");
        assert_eq!(reads.len(), 1);
        assert_eq!(reads[0].name(), "missing");
        assert_eq!(reads[0].spelling(), "$missing");
        assert_eq!(
            reads[0].source(),
            tokens.words()[1].sole_variable_substitution().unwrap().1
        );
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .execution_is_unknown()
        );
        let mut changed = tokens.clone();
        changed.variable_accesses[0].original_spelling = "$different".to_owned();
        assert!(
            changed
                .source_binding
                .as_ref()
                .unwrap()
                .declaration_read_occurrences(registry, &changed)
                .unwrap()
                .is_empty()
        );
        for source in [
            "proc p {missing} {return $missing}",
            "proc p {} {global missing; return $missing}",
            "proc p {} {namespace upvar ::ns a missing; return $missing}",
        ] {
            let (_, tokens) =
                original_tokens(source, "return $missing", source.find("return").unwrap());
            assert!(
                tokens
                    .source_binding
                    .as_ref()
                    .unwrap()
                    .declaration_read_occurrences(registry, &tokens)
                    .unwrap()
                    .is_empty(),
                "{source}"
            );
        }
        let replaced = "proc return args {}; proc p {} {return $missing}";
        let (_, tokens) = original_tokens(
            replaced,
            "return $missing",
            replaced.rfind("return").unwrap(),
        );
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .declaration_read_occurrences(registry, &tokens)
                .is_none()
        );
    }

    #[test]
    fn conditional_index_layout_keeps_native_compiler_capture_independent() {
        // Native original pair: C8.6 compiles lsetList before the unknown argv
        // substitution; Jim retains its generic script dispatch. Both bodies
        // fail at the unknown child, without entering the outer lset handler.
        // Evidence: .proofs/2286-command-declaration-graph/native-lset-pair.json.
        let source = "proc p {j v} {lset l $j [unknown]}";
        for (profile, normal) in [("jim", false), ("tcl8.6", true)] {
            let (bindings, tokens) = original_tokens_in(
                profile,
                source,
                "lset l $j [unknown]",
                source.find("lset").unwrap(),
            );
            let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
            let advice = crate::registry_invocation::conditional_index_access_advice(
                registry, None, &tokens,
            )
            .unwrap_or_else(|| {
                panic!("{profile}: original single-index layout is conditional diagnostic advice")
            });
            assert_eq!(
                advice.kind,
                crate::registry_invocation::NormalIndexAccessKind::ListWrite
            );
            assert_eq!(advice.container, "l");
            assert_eq!(advice.index, "$j");
            assert!(
                bindings
                    .declaration_operand_layout_advice(&tokens)
                    .is_some()
            );
            assert_eq!(
                crate::registry_invocation::normal_representation_invocation(
                    registry, None, &tokens
                )
                .is_some(),
                normal,
                "{profile}"
            );
        }
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let replaced = "proc lset args {}; proc p {j v} {lset l $j $v}";
        let (_, tokens) =
            original_tokens(replaced, "lset l $j $v", replaced.rfind("lset").unwrap());
        assert!(
            crate::registry_invocation::conditional_index_access_advice(registry, None, &tokens)
                .is_none()
        );
    }

    #[test]
    fn source_callee_layout_is_distinct_from_registry_metadata() {
        let source = "proc run {cmd} {$cmd 5}; proc d {} {run notacommand}";
        let (bindings, tokens) =
            original_tokens(source, "run notacommand", source.rfind("run").unwrap());
        assert!(
            bindings
                .declaration_operand_layout_advice(&tokens)
                .is_none()
        );
        let call = bindings
            .declaration_call_layout_advice(&tokens)
            .expect("original source-backed call layout");
        let [callee] = call.targets() else {
            panic!("one original source declaration");
        };
        assert!(!callee.registry_backed);
        assert_eq!(callee.kind, super::super::BindingKind::Proc);
        assert!(callee.implementation_allocation.is_some());
        let mut changed = tokens;
        changed.synthetic = Some(crate::ir::SyntheticMarker::IterationBindings(None));
        assert!(bindings.declaration_call_layout_advice(&changed).is_none());
        let source = "proc d {} {notacommand VALUE}";
        let (bindings, tokens) = original_tokens(
            source,
            "notacommand VALUE",
            source.find("notacommand").unwrap(),
        );
        assert!(bindings.declaration_call_layout_advice(&tokens).is_none());
    }

    #[test]
    fn declaration_layout_keeps_its_original_table_without_a_compiler_visit() {
        let source = "proc p {mode} {switch -- $mode {a {puts A} default {puts D}}}";
        let body = "switch -- $mode {a {puts A} default {puts D}}";
        let (bindings, tokens) = original_tokens(source, body, source.find("switch").unwrap());
        let advice = bindings.declaration_operand_layout_advice(&tokens);
        let advice = advice.expect("original declaration-owned operand layout");
        assert!(!advice.targets().is_empty());
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .compiler_lookup_state
                .is_none()
        );
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(!binding.execution_is_unknown());
        assert_eq!(
            binding.runtime_reachability(),
            super::super::SourceRuntimeReachability::Reached
        );
        assert_eq!(
            binding
                .proved_execution_target()
                .map(|target| target.command.as_str()),
            Some("::switch")
        );
        assert!(bindings.final_state.baseline.native_entry.is_none());
        let mut changed = tokens.clone();
        changed.synthetic = Some(crate::ir::SyntheticMarker::IterationBindings(None));
        assert!(
            bindings
                .declaration_operand_layout_advice(&changed)
                .is_none()
        );
        let source = "proc switch args {}; proc p {mode} {switch -- $mode {a {puts A}}}";
        let (bindings, tokens) = original_tokens(
            source,
            "switch -- $mode {a {puts A}}",
            source.rfind("switch").unwrap(),
        );
        assert!(
            bindings
                .declaration_operand_layout_advice(&tokens)
                .is_none()
        );
    }
}

#[cfg(test)]
mod original_nested_root_activation_purpose_tests {
    use super::*;

    fn assert_original_nested_eval_activation(
        source: &str,
        expected: &str,
        materialised: bool,
        frame: &crate::var_resolve::VariableExecutionFrame,
        config: tcl_lexer::LexerConfig,
        context: &tcl_registry::model::ContextRegistry,
        options: super::super::SourceAnalysisOptions<'_>,
    ) {
        let image = tcl_lexer::SourceImage::document(source);
        let bindings = SourceCommandBindings::analyse_image_in_frame_with_options(
            &image,
            frame,
            config,
            context.commands(),
            options,
        )
        .unwrap();
        let origin = Arc::clone(bindings.root_origin.as_ref().unwrap());
        let site = CommandAllocationSite {
            source: Arc::clone(&origin),
            offset: 0,
        };
        let entered = bindings.executed_script_at(&site, 0).unwrap();
        assert_eq!(entered.text.try_text().unwrap(), expected);
        if materialised {
            assert_eq!(entered.base(), 0);
            assert!(matches!(
                entered.mapping,
                super::super::ExecutedScriptMapping::Materialised
            ));
            assert_ne!(entered.origin, origin);
        }
        let body_rows = bindings
            .declaration_layouts
            .get(&CommandAllocationSite {
                source: Arc::clone(&entered.origin),
                offset: entered.base(),
            })
            .unwrap();
        let activation = body_rows
            .iter()
            .find(|row| {
                matches!(
                    row.entry.as_ref(),
                    OriginalDiagnosticFrameEntry::ScriptActivation { .. }
                )
            })
            .unwrap();
        assert_eq!(activation.entry.source(), entered);
        assert_eq!(activation.entry.frame().layout(), frame);
        assert_eq!(activation.config, config);
        assert_eq!(activation.words[0].legacy_text(), "set");
        assert!(
            matches!(activation.entry.as_ref(), OriginalDiagnosticFrameEntry::ScriptActivation {
    config: actual, ..
} if *actual == config)
        );
        assert!(activation.snapshot.state.baseline.native_entry.is_none());
        assert!(
            bindings
                .declaration_layouts
                .get(&site)
                .unwrap()
                .iter()
                .any(|row| matches!(
                    row.entry.as_ref(),
                    OriginalDiagnosticFrameEntry::RootScript { .. }
                ))
        );
    }

    #[test]
    fn original_nested_parser_keeps_entered_root_frame_separate_from_whole_input() {
        // naming.source.original-configured-script-completeness

        // A genuine eval observation supplies its frame; NestedScript supplies grammar only.
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let config = tcl_lexer::LexerConfig {
            leading_bom: tcl_lexer::LeadingBom::Skip,
            strict_quoting: true,
            braced_var: tcl_lexer::BracedVarStyle::FirstClose,
            ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
        };
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            config,
        );
        let frame = crate::var_resolve::VariableExecutionFrame::Global;
        let options = super::super::SourceAnalysisOptions::for_logical_source(&input).unwrap();
        for (source, expected, materialised) in [
            ("eval {set x VALUE; list $x}", "set x VALUE; list $x", false),
            ("eval {set} {x VALUE}", "set x VALUE", true),
        ] {
            assert_original_nested_eval_activation(
                source,
                expected,
                materialised,
                &frame,
                config,
                &context,
                options,
            );
        }

        let body_at_zero = tcl_lexer::SourceImage::document("set x VALUE");
        let nested = SourceCommandBindings::analyse_image_in_frame_with_purpose(
            &body_at_zero,
            &frame,
            config,
            context.commands(),
            options,
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
        )
        .unwrap();
        assert!(nested.points.iter().any(|point| point.offset == 0));
        assert!(
            nested
                .declaration_layouts
                .values()
                .flatten()
                .all(|row| !matches!(
                    row.entry.as_ref(),
                    OriginalDiagnosticFrameEntry::RootScript { .. }
                ))
        );
        assert!(nested.original_script_activations.is_empty());
        let whole = SourceCommandBindings::analyse_image_in_frame_with_options(
            &body_at_zero,
            &frame,
            config,
            context.commands(),
            options,
        )
        .unwrap();
        assert!(
            whole
                .declaration_layouts
                .values()
                .flatten()
                .any(|row| matches!(
                    row.entry.as_ref(),
                    OriginalDiagnosticFrameEntry::RootScript { .. }
                ))
        );
    }
}

#[cfg(test)]
#[path = "logical_purpose_tests.rs"]
mod logical_purpose_tests;

#[cfg(test)]
mod original_logical_declaration_receipt_tests {
    use crate::analyser::ResolvedAnalysisInput;
    use crate::ir::CommandTokens;

    fn original_statement(source: &str, selected: &str) -> (ResolvedAnalysisInput, CommandTokens) {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_known_environment("tcl8.6")
                .unwrap()
                .default_context_registry(),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        let mut lowerer = crate::lowering::Lowerer::with_config(
            input.borrowed_context_registry().commands(),
            input.lexer_config(),
        )
        .with_resolved_analysis_input(input.clone());
        lowerer.lower(source);
        let module = lowerer.finish_module(source);
        assert_eq!(module.source_metadata_input.as_ref(), Some(&input));
        assert_eq!(module.lexer_config, input.lexer_config());
        let offset = u32::try_from(source.rfind(selected).unwrap()).unwrap();
        let mut found = Vec::new();
        let mut collect = |statement: &crate::ir::Statement| {
            if statement.span().start() == offset {
                found.push(
                    statement
                        .tokens()
                        .expect("original statement vector")
                        .clone(),
                );
            }
        };
        crate::ir::for_each_statement(&module.top_level, &mut collect);
        for procedure in module.procedures.values() {
            crate::ir::for_each_statement(&procedure.body, &mut collect);
        }
        assert_eq!(found.len(), 1);
        (input, found.pop().unwrap())
    }

    #[test]
    fn original_logical_nested_declaration_retains_availability_without_dispatch() {
        // naming.compiler.original-positioned-callee-body-availability

        for (source, selected, prefix) in [
            (
                "proc subject {} {package require Tcl}",
                "package require Tcl",
                0,
            ),
            (
                "interp alias {} ensure {} package require; proc subject {} {ensure Tcl}",
                "ensure Tcl",
                1,
            ),
        ] {
            let (input, tokens) = original_statement(source, selected);
            let binding = tokens.source_binding.as_ref().unwrap();
            assert!(binding.lookup_state.is_none());
            assert!(binding.unknown);
            assert!(binding.targets.is_empty());
            assert!(binding.original_logical_declaration_targets.is_some());
            let targets = binding
                .original_logical_declaration_call_targets(
                    &tokens,
                    &input,
                    input.borrowed_context_registry().commands(),
                )
                .expect("closed conditional original declaration");
            let [target] = targets.as_slice() else {
                panic!("one selected source target")
            };
            assert!(target.registry_backed);
            assert_eq!(target.prepended.len(), prefix);
            assert!(target.implementation_allocation.is_none());
            assert!(binding.proved_target().is_none());
            assert!(binding.proved_execution_target().is_none());
            assert!(binding.proved_handler_target().is_none());
            assert!(binding.native_compiler_admission.is_none());
            assert!(binding.native_compilation_admission.is_none());
        }
    }

    fn assert_changed_declared_call_inputs(
        binding: &crate::command_binding::SourceInvocationBinding,
        tokens: &CommandTokens,
        input: &ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
    ) {
        for axis in [0, 1] {
            let mut config = input.lexer_config();
            if axis == 0 {
                config.leading_bom = match config.leading_bom {
                    tcl_lexer::LeadingBom::Content => tcl_lexer::LeadingBom::Skip,
                    tcl_lexer::LeadingBom::Skip => tcl_lexer::LeadingBom::Content,
                };
            } else {
                config.strict_quoting = !config.strict_quoting;
            }
            let changed = ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                input.context_registry(),
                config,
            );
            assert!(
                binding
                    .original_logical_declaration_call_targets(tokens, &changed, registry)
                    .is_none()
            );
        }
    }

    fn assert_changed_original_call_refused(
        changed: &CommandTokens,
        input: &ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
    ) {
        assert!(
            changed
                .source_binding
                .as_ref()
                .unwrap()
                .original_logical_declaration_call_targets(changed, input, registry,)
                .is_none()
        );
    }

    #[test]
    fn original_logical_declaration_receipt_rejects_changed_inputs_rows_and_world() {
        // naming.compiler.original-positioned-callee-body-availability

        let (input, tokens) = original_statement(
            "proc subject {} {package require Tcl}",
            "package require Tcl",
        );
        let registry = input.borrowed_context_registry().commands();
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(
            binding
                .original_logical_declaration_call_targets(&tokens, &input, registry)
                .is_some()
        );
        assert_changed_declared_call_inputs(binding, &tokens, &input, registry);
        let other_input = ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            tcl_registry::model::ingress::resolve_known_environment("tcl9.0")
                .unwrap()
                .default_context_registry(),
            input.lexer_config(),
        );
        assert!(
            binding
                .original_logical_declaration_call_targets(
                    &tokens,
                    &other_input,
                    other_input.borrowed_context_registry().commands(),
                )
                .is_none()
        );
        let (_, foreign) = original_statement(
            "proc another {} {package require Tcl}",
            "package require Tcl",
        );
        let mut changed = tokens.clone();
        changed
            .source_binding
            .as_mut()
            .unwrap()
            .declaration_layout_observations = foreign
            .source_binding
            .as_ref()
            .unwrap()
            .declaration_layout_observations
            .clone();
        assert_changed_original_call_refused(&changed, &input, registry);
        let mut changed = tokens.clone();
        changed.source_binding.as_mut().unwrap().unknown = false;
        assert_changed_original_call_refused(&changed, &input, registry);
        let mut changed = tokens.clone();
        changed.source_binding.as_mut().unwrap().may_be_absent = true;
        assert_changed_original_call_refused(&changed, &input, registry);
        let mut changed = tokens.clone();
        changed
            .source_binding
            .as_mut()
            .unwrap()
            .join(&super::super::SourceInvocationBinding::default());
        assert!(
            changed
                .source_binding
                .as_ref()
                .unwrap()
                .original_logical_declaration_targets
                .is_none()
        );
        assert_changed_original_call_refused(&changed, &input, registry);
        let mut changed = tokens.clone();
        changed.synthetic = Some(crate::ir::SyntheticMarker::EvaluatedArguments);
        assert!(
            binding
                .original_logical_declaration_call_targets(&changed, &input, registry)
                .is_none()
        );
        let mut changed = tokens.clone();
        changed.word_exprs.pop();
        assert!(
            binding
                .original_logical_declaration_call_targets(&changed, &input, registry)
                .is_none()
        );
    }

    #[test]
    fn original_logical_declaration_receipt_keeps_reached_and_opaque_guards() {
        // naming.compiler.original-positioned-callee-body-availability

        let (input, tokens) = original_statement("package require Tcl", "package require Tcl");
        let registry = input.borrowed_context_registry().commands();
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(binding.lookup_state.is_some());
        assert!(binding.original_logical_declaration_targets.is_none());
        assert!(
            binding
                .original_logical_declaration_call_targets(&tokens, &input, registry)
                .is_some()
        );
        for axis in 0..3 {
            let mut changed = tokens.clone();
            let binding = changed.source_binding.as_mut().unwrap();
            match axis {
                0 => binding.unknown = true,
                1 => binding.may_be_absent = true,
                _ => binding.targets.clear(),
            }
            assert!(
                changed
                    .source_binding
                    .as_ref()
                    .unwrap()
                    .original_logical_declaration_call_targets(&changed, &input, registry,)
                    .is_none()
            );
        }
        let (input, tokens) = original_statement(
            "proc subject {} {$callback; package require Tcl}",
            "package require Tcl",
        );
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(binding.original_logical_declaration_targets.is_none());
        assert!(
            binding
                .original_logical_declaration_call_targets(
                    &tokens,
                    &input,
                    input.borrowed_context_registry().commands(),
                )
                .is_none()
        );
        assert!(binding.proved_execution_target().is_none());
    }
}

#[cfg(test)]
mod authored_source_parent_tests {
    use super::*;

    #[test]
    fn authored_source_parent_reborrow_keeps_allocated_and_uninstalled_body_owners() {
        let environment = tcl_registry::model::ingress::resolve_environment("tcl8.6");
        let context = environment.default_context_registry();
        let registry = context.commands();
        let profile = environment.unit_profile();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        for (prefix, allocated) in [("", true), ("rename $old decl\n", false)] {
            let source = format!("{prefix}proc kept {{x}} {{list retained}}");
            let unit = crate::compilation_unit::CompilationUnit::build_for_with_config(
                &source, registry, false, config,
            );
            let module = &unit.ir_module;
            let (&id, _) = module
                .original_authored_declaration_body_units
                .iter()
                .find(|(_, card)| card.name() == "kept")
                .unwrap();
            let procedure = &module.body_units[&id];
            let tokens = procedure
                .body
                .retained_source_tokens_for_statement(&procedure.body.statements[0])
                .unwrap();
            let scope = tokens
                .source_binding
                .as_ref()
                .unwrap()
                .original_authored_source_body_scope_for_name_roles(tokens, registry)
                .unwrap();
            let bindings = module
                .retained_source_bindings
                .as_ref()
                .unwrap()
                .positioned_for_module(module, registry)
                .unwrap();
            let site = CommandAllocationSite {
                source: Arc::clone(bindings.source_origin().unwrap()),
                offset: tokens.words()[0].source().span.start(),
            };
            let observations = bindings.declaration_layouts.get(&site).unwrap();
            for row in original_declaration_layouts(observations).unwrap() {
                match row.entry.as_ref() {
                    OriginalDiagnosticFrameEntry::Body(body) => {
                        assert!(allocated, "{prefix:?}");
                        assert_eq!(
                            &body.allocation().site,
                            scope.original_parent_site().unwrap()
                        );
                    }
                    OriginalDiagnosticFrameEntry::DeclaredProcedure(body) => {
                        assert!(!allocated, "{prefix:?}");
                        assert_eq!(
                            body.declaration_site(),
                            scope.original_parent_site().unwrap()
                        );
                    }
                    _ => panic!("actual procedure source owner"),
                }
            }
            let selected = bindings
                .original_authored_declaration_source_tokens(tokens, scope, registry)
                .expect("genuine original parent/source row");
            let binding = selected.source_binding.as_ref().unwrap();
            assert!(
                binding
                    .original_authored_declaration_scalar_layout(&selected, registry)
                    .is_some()
            );
            assert!(binding.execution_is_unknown());
            assert!(binding.proved_execution_target().is_none());
            assert!(
                crate::registry_invocation::normal_representation_invocation_with_metadata_context(
                    registry, None, &selected,
                )
                .is_none()
            );
            assert!(
                crate::registry_invocation::admitted_native_compiler_invocation(
                    registry, None, &selected,
                )
                .is_none()
            );
            let mut withdrawn = bindings.clone();
            withdrawn.declaration_layouts.remove(&site);
            assert!(
                withdrawn
                    .original_authored_declaration_source_tokens(tokens, scope, registry,)
                    .is_none()
            );
        }
    }
}
