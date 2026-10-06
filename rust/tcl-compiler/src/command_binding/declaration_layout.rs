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
    RootScript {
        source: super::ExecutedScriptSource,
        frame: crate::var_resolve::VariableExecutionFrame,
        namespace: super::SourceNamespaceKey,
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
            Self::RootScript { namespace, .. } => Some(namespace),
        };
        SourceCommandBindings::context_owns_frame(context, self.frame(), namespace)
    }

    pub(super) fn source(&self) -> &super::ExecutedScriptSource {
        match self {
            Self::Body(body) => body.source(),
            Self::DeclaredReceiver(body) => body.source(),
            Self::RootScript { source, .. } => source,
        }
    }

    pub(super) fn frame(&self) -> &crate::var_resolve::VariableExecutionFrame {
        match self {
            Self::Body(body) => body.frame(),
            Self::DeclaredReceiver(body) => body.preview_frame(),
            Self::RootScript { frame, .. } => frame,
        }
    }

    pub(super) fn parameters(&self) -> &[tcl_syntax::formal_params::FormalParameter] {
        match self {
            Self::Body(body) => body.parameters(),
            Self::DeclaredReceiver(body) => body.parameters(),
            Self::RootScript { .. } => &[],
        }
    }

    pub(super) fn owns_source(&self, origin: &Arc<super::SourceOriginId>, offset: u32) -> bool {
        let source = self.source();
        &source.origin == origin
            && offset >= source.base()
            && u64::from(offset) < u64::from(source.base()) + source.text.len() as u64
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
            Self::RootScript {
                frame: expected, ..
            } => expected == frame,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeclarationLayoutObservation {
    issuer: DeclarationLayoutIssuer,
    pub(super) entry: Arc<OriginalDiagnosticFrameEntry>,
    pub(super) snapshot: Arc<SourceLookupSnapshot>,
    pub(super) namespace: super::SourceNamespaceKey,
    pub(super) config: tcl_lexer::LexerConfig,
    pub(super) words: Arc<[crate::ir::WordExpr]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DeclarationLayoutIssuer {
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
    let first = original.clone().next()?;
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
        binding.declaration_flow_inventory =
            binding.declaration_layout_observations.as_ref().map(|_| {
                Arc::new(super::declaration_flow::DeclarationFlowInventory::new(
                    self.declaration_layouts.shared(),
                    self.variable_accesses.shared(),
                    self.origin_variable_accesses.shared(),
                    self.points.shared(),
                    self.origin_points.shared(),
                ))
            });
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
                        || crate::script_binds::script_image_binds_name(
                            &observation.entry.source().text,
                            name,
                            crate::script_binds::Ownership::ScopeAliases,
                            registry,
                            config,
                        )
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
        let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
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
                || crate::script_binds::script_image_binds_name(
                    &observation.entry.source().text,
                    &name,
                    crate::script_binds::Ownership::ScopeAliases,
                    registry,
                    config,
                )
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

    fn original_declared_layout_advice(
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
    fn repeated_or_replaced_declarations_do_not_share_original_layout_issuers() {
        let body = "expr {$n + 1}";
        for source in [
            "foreach generation {A B} {proc p {n} {expr {$n + 1}}}; p 1; p 2",
            "proc expr args {return CUSTOM}; proc p {n} {expr {$n + 1}}; p 1; p 2",
        ] {
            let (_, tokens) = original_tokens(source, body, source.find(body).unwrap());
            let binding = tokens.source_binding.as_ref().unwrap();
            assert!(
                binding.declaration_operand_layout_advice(&tokens).is_none(),
                "{source}"
            );
        }
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
    fn declaration_read_occurrences_keep_missing_reads_without_physical_ssa() {
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
