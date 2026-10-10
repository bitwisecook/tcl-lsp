//! Original operand layouts observed in an original body's own frame.
//!
//! A layout observation precedes operand evaluation. It preserves candidate
//! lookup and logical grammar for diagnostics. Declared frames and original
//! entered frames retain their independent identities; this never publishes compiler
//! admission, successful dispatch, an entered body, or closed runtime effects.

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

impl OriginalDiagnosticFrameEntry {
    pub(super) fn owns_original_context(
        &self,
        context: &crate::var_resolve::ResolveContext,
    ) -> bool {
        let namespace = match self {
            Self::Body(body) => body.namespace_context(),
            Self::DeclaredReceiver(_) => None,
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
            Self::DeclaredProcedure(body) => &body.source,
            Self::RootScript { source, .. } | Self::ScriptActivation { source, .. } => source,
        }
    }

    pub(super) fn frame(&self) -> &crate::var_resolve::VariableExecutionFrame {
        match self {
            Self::Body(body) => body.frame(),
            Self::DeclaredReceiver(body) => body.preview_frame(),
            Self::DeclaredProcedure(body) => &body.frame,
            Self::RootScript { frame, .. } | Self::ScriptActivation { frame, .. } => frame,
        }
    }

    pub(super) fn parameters(&self) -> &[tcl_syntax::formal_params::FormalParameter] {
        match self {
            Self::Body(body) => body.parameters(),
            Self::DeclaredReceiver(body) => body.parameters(),
            Self::DeclaredProcedure(body) => &body.parameters,
            Self::RootScript { .. } | Self::ScriptActivation { .. } => &[],
        }
    }

    pub(super) fn original_formal_topology(
        &self,
    ) -> Option<&super::formal_topology::OriginalFormalTopology> {
        match self {
            Self::Body(body) => body.original_formal_topology(),
            Self::DeclaredReceiver(body) => body.original_formal_topology(),
            Self::DeclaredProcedure(body) => body.original_parameters.as_ref(),
            Self::RootScript { .. } | Self::ScriptActivation { .. } => None,
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
}

/// Select the immutable declaration owner without joining entered activations.
/// A conflicting original receipt withdraws the entire declaration projection.
pub(super) fn original_declaration_layouts(
    observations: &[DeclarationLayoutObservation],
) -> Option<impl Iterator<Item = &DeclarationLayoutObservation> + Clone> {
    let original = observations
        .iter()
        .filter(|observation| observation.issuer == DeclarationLayoutIssuer::OriginalDeclaration);
    #[cfg(debug_assertions)]
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_DECLARATION").is_some()
        && original.clone().next().is_none()
    {
        eprintln!(
            "ORIGINAL_DECLARATION stage=no-original observations={}",
            observations.len()
        );
    }
    let first = original.clone().next()?;
    #[cfg(debug_assertions)]
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_DECLARATION").is_some() {
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
        observation.entry != first.entry
            || observation.config != first.config
            || observation.words != first.words
            || observation.snapshot.state.variable_frame != *observation.entry.frame()
            || observation.snapshot.state.current_source_origin.as_ref()
                != Some(&observation.entry.source().origin)
            || !observation
                .entry
                .owns_original_context(&observation.snapshot.state.source_variables)
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

/// Immutable original local-frame naming owner. This is a conditional source
/// declaration identity, not an entered activation or a physical local table.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceOriginalVariableFrame {
    source: super::ExecutedScriptSource,
    frame: crate::var_resolve::VariableExecutionFrame,
    config: tcl_lexer::LexerConfig,
}

impl SourceOriginalVariableFrame {
    /// Genuine original procedure/method frame selected by its body recipe.
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

impl SourceCommandBindings {
    pub(super) fn retain_original_script_activation(
        &mut self,
        image: &tcl_lexer::SourceImage,
        base: u32,
        state: &ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) {
        if !matches!(
            state.variable_frame.layout(),
            crate::var_resolve::VariableExecutionFrame::NamespaceActivation { .. }
        ) || &state.variable_frame != context.frame
        {
            return;
        }
        let Some(origin) = state.current_source_origin.as_ref() else {
            return;
        };
        if self.root_origin.as_ref() != Some(origin) {
            return;
        }
        let namespace = context.namespace_identity();
        if state.source_variables.namespace_identity.as_ref() != Some(&namespace) {
            return;
        }
        let Some(source) =
            super::ExecutedScriptSource::contiguous_image(Arc::clone(origin), image.clone(), base)
        else {
            return;
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
        #[cfg(test)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_SYMBOLS").is_some() {
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
                #[cfg(test)]
                if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_SYMBOLS").is_some() {
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
        if span != declaration.span()
            && !matches!(protocol.variable_root_geometry(NativeNameContext::root(), selected.root().selected()), NativeVariableRootGeometry::Local(ref name) if name == alias.local())
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
            for row in rows {
                if !input.is_current(&row.snapshot.state.source_variables)
                    || !original_local_primary_is_byte_unique(
                        &row.snapshot.state,
                        input.policy(),
                        alias.local().as_bytes(),
                    )
                {
                    return false;
                }
                found = true;
            }
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
                    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_SYMBOLS").is_some() {
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
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_COMPLETION").is_some() {
            eprintln!(
                "ORIGINAL_VARIABLE_FRAME offset={offset} layouts={} deferred={}",
                self.declaration_layouts.len(),
                self.deferred.len(),
            );
        }
        for observations in self.declaration_layouts.values() {
            #[cfg(debug_assertions)]
            if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_COMPLETION").is_some() {
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
            };
            match &found {
                Some(previous) if previous.source.text.len() < frame.source.text.len() => {}
                Some(previous) if previous.source.text.len() == frame.source.text.len() => {
                    if previous != &frame {
                        #[cfg(debug_assertions)]
                        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_COMPLETION").is_some()
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
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_COMPLETION").is_some() {
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
        let snapshot = Arc::new(SourceLookupSnapshot::in_realm(state.clone(), context.realm));
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
            snapshot: Arc::new(SourceLookupSnapshot::in_realm(state.clone(), context.realm)),
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

impl super::SourceInvocationBinding {
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
        let config = tcl_lexer::LexerConfig::from_grammar(layout.dialect().lexer_grammar);
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
            if name.contains("::")
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
        let dialect = layout.dialect();
        let crate::registry_invocation::EffectiveInvocationWord::Literal(name) =
            crate::registry_invocation::effective_invocation_word(
                word,
                dialect.lexer_grammar.escapes,
                dialect.word_values,
            )
        else {
            return None;
        };
        if name.contains("::")
            || tcl_syntax::naming::split_array_name_braced_for_style(
                &name,
                true,
                dialect.lexer_grammar.braced_var,
            )
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
                        crate::script_binds::Ownership::ScopeAliases,
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
            let project = if source_calls {
                super::original_declaration_call_layout_advice
            } else {
                super::original_operand_layout_advice
            };
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
        let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
        let dialect = tcl_registry::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::of_dialect_name(Some(profile)).unwrap(),
        );
        let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
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
        // docs/design/analysis/name-resolution-proofs/original-body-cursor-frame.md
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
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
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
    fn original_destruction_operand_keys_follow_the_retained_alias_point() {
        // Implementation contract: naming.variable.original-destruction-operand-keys
        // docs/design/analysis/name-resolution-proofs/original-destruction-operand-keys.md
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
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
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
        if advice.is_none() {
            let binding = tokens.source_binding.as_ref().unwrap();
            eprintln!(
                "declaration layout: site={:?} frame={:?} observations={} entry={:?}",
                binding.invocation_site().map(|site| site.offset),
                binding.variable_frame,
                bindings.declaration_layouts.len(),
                binding
                    .invocation_site()
                    .and_then(|site| bindings.conditional_body_entry_at(&site.source, site.offset))
            );
            for (site, observations) in &bindings.declaration_layouts {
                for observation in observations {
                    eprintln!(
                        "declaration observation: site={} origin_matches={} frame_matches={} owns_source={} owns_invocation={} dialect={:?} targets={:?}",
                        site.offset,
                        observation.snapshot.state.current_source_origin.as_ref()
                            == Some(&site.source),
                        observation.snapshot.state.variable_frame == binding.variable_frame,
                        observation.entry.owns_source(&site.source, site.offset),
                        observation.entry.owns_invocation(binding),
                        observation
                            .snapshot
                            .state
                            .source_variables
                            .invocation_dialect,
                        observation
                            .snapshot
                            .state
                            .targets("switch", &observation.namespace)
                            .iter()
                            .map(|target| (&target.command, target.registry_backed))
                            .collect::<Vec<_>>()
                    );
                }
            }
        }
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
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .execution_is_unknown()
        );
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
