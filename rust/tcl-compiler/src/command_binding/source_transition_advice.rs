// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Authored command transitions retain source advice independently of execution.

mod body_effects;
mod interpreter_handle;
pub use interpreter_handle::OriginalSourceInterpreterHandleBody;
mod interpreter_path;
pub use interpreter_path::OriginalSourceInterpreterPathBinding;
mod logical_body;
mod logical_declaration;
mod native_baseline;
mod produced_prefix;
mod registered_instance;
mod source_callback;
mod source_class;
mod source_class_publications;
mod source_class_reference;
mod source_configured_class;
pub use source_callback::{
    OriginalSourceCallbackProcedureLookup, OriginalSourceCallbackProcedureRefusal,
    OriginalSourceCallbackProcedureTarget, OriginalSourceCallbackProcedureTargetKind,
    OriginalSourceCallbackRegistration,
};
pub use source_class_publications::{
    OriginalSourceClassCandidate, OriginalSourceClassPublication, OriginalSourceClassPublications,
};
pub use source_class_reference::OriginalSourceClassReference;
pub use source_configured_class::OriginalSourceConfiguredClassReference;
mod source_instance;
mod source_procedure;
mod source_procedure_publications;
pub(crate) use produced_prefix::original_single_command_substitution_words;
pub use produced_prefix::{
    OriginalSourceAuthoredCommandPrefix, OriginalSourceProducedCommandPrefix,
};
pub use registered_instance::{
    OriginalSourceRegisteredHandleBinding, OriginalSourceRegisteredInstance,
    OriginalSourceRegisteredInstanceWords,
};
pub use source_class::{
    OriginalSourceClassDeclaration, OriginalSourceConstructorCall, OriginalSourceConstructorShape,
};
pub use source_instance::{
    OriginalSourceClassHandleBinding, OriginalSourceClassInstance, OriginalSourceClassInstanceWords,
};
pub use source_procedure_publications::{
    OriginalSourceProcedureCandidate, OriginalSourceProcedurePublication,
    OriginalSourceProcedurePublications,
};

use crate::signature_scan::scope::{SignatureSourceNameInput, SignatureSourceNameKey};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;
use tcl_core_types::{ByteCommandSlot, ByteNamespacePath, NameBytes};
use tcl_lexer::{LexerConfig, NativeWord, SourceImage};
use tcl_registry::model::{ContextRegistry, ResolvedContext};
use tcl_registry::{
    AliasTargetLookup, CommandBindingTransition, InvocationDialect, StateTransition,
    TransitionSubject,
};
use tcl_syntax::naming::{NamePolicyProtocol, NativeNameContext, NativeNameProtocol};

/// A genuine original Logical source word and its independently supported
/// ASCII presentation. No Native name recipe, value object or slot is issued.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalLogicalSourceNameInput {
    original: NativeWord,
    bytes: Arc<[u8]>,
}
impl OriginalLogicalSourceNameInput {
    /// Whole original word, preserving channel, grouping and full configuration.
    #[must_use]
    pub const fn original_word(&self) -> &NativeWord {
        &self.original
    }
    /// Shared source presentation, independently of execution value materialisation.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Source advice retains the positively selected naming purpose. A Logical
/// producer cannot be projected into a Native naming key or policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceAdviceNameInput {
    /// Independently selected Native authored naming input.
    Native(SignatureSourceNameInput),
    /// Positively selected Logical whole-source metadata input.
    Logical(OriginalLogicalSourceNameInput),
}
impl SourceAdviceNameInput {
    /// Purpose-selected source units; these alone carry no runtime grant.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        match self {
            Self::Native(input) => input.bytes(),
            Self::Logical(input) => input.bytes(),
        }
    }
    /// Genuine original word, without reconstructing a prefix's source position.
    #[must_use]
    pub fn original_word(&self) -> Option<&NativeWord> {
        match self {
            Self::Native(input) => Some(input.original_word_key()?.original_word()),
            Self::Logical(input) => Some(input.original_word()),
        }
    }
    /// Native key only when that independent issuer supplied it.
    #[must_use]
    pub const fn native_input(&self) -> Option<&SignatureSourceNameInput> {
        match self {
            Self::Native(input) => Some(input),
            Self::Logical(_) => None,
        }
    }
    /// Logical original source input only when its explicit domain was selected.
    #[must_use]
    pub const fn logical_input(&self) -> Option<&OriginalLogicalSourceNameInput> {
        match self {
            Self::Logical(input) => Some(input),
            Self::Native(_) => None,
        }
    }
}

#[derive(Debug, Clone)]
enum AdviceNamingPolicy {
    Native(NamePolicyProtocol),
    Logical(crate::analyser::ResolvedAnalysisInput),
}
impl AdviceNamingPolicy {
    fn native(&self) -> Option<NamePolicyProtocol> {
        match self {
            Self::Native(policy) => Some(*policy),
            Self::Logical(_) => None,
        }
    }
    fn logical_slot(bytes: &[u8]) -> Option<ByteCommandSlot> {
        std::str::from_utf8(bytes).ok()?;
        if bytes.contains(&0) {
            return None;
        }
        tcl_syntax::naming::authored_source_command_slot(&ByteNamespacePath::root(), bytes)
    }
    fn namespace_path(&self, bytes: &[u8]) -> Option<ByteNamespacePath> {
        match self {
            Self::Native(policy) => policy
                .recipe()
                .namespace_address_path(root_context(*policy), bytes)
                .ok(),
            Self::Logical(_) => Some(ByteNamespacePath::from_segments(
                tcl_syntax::naming::qualifier_segments(bytes),
            )),
        }
    }
    fn publication_slot(
        &self,
        bytes: &[u8],
        purpose: AdvicePublicationPurpose,
    ) -> Option<ByteCommandSlot> {
        match self {
            Self::Logical(_) => Self::logical_slot(bytes),
            Self::Native(policy) => match purpose {
                AdvicePublicationPurpose::Define => policy
                    .recipe()
                    .command_publication_slot(root_context(*policy), bytes)
                    .ok(),
                AdvicePublicationPurpose::Move => policy
                    .recipe()
                    .rename_destination_slot(root_context(*policy), bytes)
                    .ok(),
                AdvicePublicationPurpose::Alias => policy
                    .recipe()
                    .alias_publication_slot(root_context(*policy), bytes)
                    .ok(),
            },
        }
    }
    fn rename_source(&self, bytes: &[u8]) -> Option<Vec<u8>> {
        match self {
            Self::Logical(_) => Some(bytes.to_vec()),
            Self::Native(policy) => Some(
                policy
                    .recipe()
                    .rename_source_input(root_context(*policy), bytes)
                    .ok()?
                    .selected()
                    .to_vec(),
            ),
        }
    }
    fn rename_destination(&self, bytes: &[u8]) -> Option<Vec<u8>> {
        match self {
            Self::Logical(_) => Some(bytes.to_vec()),
            Self::Native(policy) => Some(
                policy
                    .recipe()
                    .rename_destination_input(root_context(*policy), bytes)
                    .ok()?
                    .selected()
                    .to_vec(),
            ),
        }
    }
}
#[derive(Clone, Copy)]
enum AdvicePublicationPurpose {
    Define,
    Move,
    Alias,
}

/// Applicability of an authored transition recipe; this is never a live table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceCommandTransitionObligation {
    /// A written transition does not prove that its handler executes successfully.
    WrittenTransitionApplicability,
    /// No actual source lookup selected this advice at the consumer's point.
    UnavailableActualLookup,
    /// Authentic closed entry rows supply only possible original source syntax.
    /// The selected handler and table at this invocation remain independent.
    NativeBaselineSourceApplicability,
    /// A prior source operation can mutate commands or namespace lookup.
    UnknownEarlierMutation,
    /// Explicit Logical source assistance, without a Native naming/value issuer.
    ExplicitLogicalSourceApplicability,
    /// Actual original declaration entry/frame and before-argv alternatives.
    /// This does not establish that the body or handler is entered.
    ConditionalDeclarationApplicability,
    /// A complete Logical source body and its authentic parent schema own
    /// this conditional header. No runtime activation or frame is supplied.
    DeferredLogicalBodyApplicability,
    /// A selected original immediate body supplies conditional source syntax,
    /// without complete operand effects, an entered frame or execution facts.
    ConditionalLogicalBodyApplicability,
    /// Original list data describes a conditional target without a deferred parent.
    AuthoredCommandPrefixApplicability,
    /// A selected list builder describes a possible future invocation only.
    ProducedCommandPrefixApplicability,
    /// Successful named factory execution remains an independent premise.
    RegisteredFactoryApplicability,
    /// An original procedure body is conditional source syntax, never an entered frame.
    OriginalProcedureBodyApplicability,
    /// Genuine retained procedure syntax is considered for a possible future
    /// entry against the final authored graph, without a written call or frame.
    FutureOriginalProcedureSourceApplicability,
    /// The original setter may retain the factory result; no cell is supplied.
    RegisteredHandleBindingApplicability,
    /// Original registration target and authored aliases remain conditional;
    /// no reached callback, future table, argument vector or entered frame.
    OriginalCallbackTargetApplicability,
    /// The selected source prefix has no callback lookup-frame descriptor.
    /// A rooted C source-name correlation does not supply the missing frame.
    UnavailableCallbackLookupFrame,
}

/// Complete source operation and the exact Registry-authored transition recipe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceCommandTransition {
    site: super::CommandAllocationSite,
    original: Arc<[NativeWord]>,
    inputs: Vec<(TransitionSubject, SourceAdviceNameInput)>,
    descriptor: String,
    transitions: tcl_registry::StateTransitions,
    selection_lineage: Vec<Arc<OriginalSourceCommandTransition>>,
}
impl OriginalSourceCommandTransition {
    /// Actual original source origin and lexical site, without a runtime point.
    #[must_use]
    pub const fn site(&self) -> &super::CommandAllocationSite {
        &self.site
    }
    /// Whole original source vector, never reconstructed from reported names.
    #[must_use]
    pub fn original_words(&self) -> &[NativeWord] {
        &self.original
    }
    /// Genuine source inputs matched to the selected transition subjects.
    #[must_use]
    pub fn inputs(&self) -> &[(TransitionSubject, SourceAdviceNameInput)] {
        &self.inputs
    }
    /// Canonical authored Registry schema, independently of dispatch.
    #[must_use]
    pub fn descriptor(&self) -> &str {
        &self.descriptor
    }
    /// Exact authored transition recipe retained by this operation.
    #[must_use]
    pub const fn transitions(&self) -> &tcl_registry::StateTransitions {
        &self.transitions
    }
    /// Genuine earlier aliases and moves that selected this operation's schema.
    /// A written operation invoked through an alias retains that producer chain.
    #[must_use]
    pub fn selection_lineage(&self) -> &[Arc<OriginalSourceCommandTransition>] {
        &self.selection_lineage
    }
    fn input(&self, subject: &TransitionSubject) -> Option<&SourceAdviceNameInput> {
        self.inputs
            .iter()
            .find(|(owned, _)| owned == subject)
            .map(|(_, input)| input)
    }
    fn selected_input<'a>(&self, subject: &'a TransitionSubject) -> Option<&'a [u8]> {
        self.input(subject)?;
        subject
            .native_bytes()
            .or_else(|| subject.literal().map(str::as_bytes))
    }
    fn is_current_interpreter(&self, subject: &TransitionSubject) -> Option<bool> {
        match subject {
            // Registry recipes may supply the current-interpreter constant;
            // it has no original word and must never acquire one.
            TransitionSubject::Literal(value) => Some(value.is_empty()),
            _ => self.input(subject).map(|input| input.bytes().is_empty()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceAdviceWord {
    pub(crate) original: NativeWord,
    pub(crate) input: Option<SourceAdviceNameInput>,
    pub(crate) value: Option<Arc<[u8]>>,
    pub(crate) origin: crate::registry_invocation::InvocationWordOrigin,
}

/// Sealed conditional source argv with its own original naming purpose and
/// optional genuine authored transition lineage.
/// This supplies no entered handler, current command table, Native, Normal,
/// reflection equivalence or syntax-compaction authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceCommandTransitionAdvice {
    site: super::CommandAllocationSite,
    original: Arc<[NativeWord]>,
    head: SourceAdviceNameInput,
    arguments: Vec<SourceAdviceWord>,
    command: String,
    context: ResolvedContext,
    registry: tcl_registry::RegistrySemanticKey,
    config: LexerConfig,
    dialect: InvocationDialect,
    roles: Option<Vec<(usize, tcl_registry::ArgRole)>>,
    lineage: Vec<Arc<OriginalSourceCommandTransition>>,
    obligations: Vec<SourceCommandTransitionObligation>,
    uncertain_operations: Vec<Arc<[NativeWord]>>,
    logical_input: Option<crate::analyser::ResolvedAnalysisInput>,
    declaration: Option<Arc<[super::declaration_layout::DeclarationLayoutObservation]>>,
    logical_body: Option<Arc<crate::registry_invocation::OriginalSourceScriptBody>>,
    procedure_body: Option<Arc<crate::registry_invocation::OriginalSourceScriptBody>>,
}
impl OriginalSourceCommandTransitionAdvice {
    /// Actual lexical source occurrence; not a fabricated dispatch point.
    #[must_use]
    pub const fn site(&self) -> &super::CommandAllocationSite {
        &self.site
    }
    /// Current call's complete unchanged original vector.
    #[must_use]
    pub fn original_words(&self) -> &[NativeWord] {
        &self.original
    }
    /// Genuine current written command head, independent of its target schema.
    #[must_use]
    pub const fn original_head(&self) -> &SourceAdviceNameInput {
        &self.head
    }
    /// Validated immutable full input for explicit Logical source advice.
    /// Native advice never supplies this carrier, and this cannot issue a
    /// Native naming policy, execution state or source-lookup authority.
    #[must_use]
    pub const fn logical_source_input(&self) -> Option<&crate::analyser::ResolvedAnalysisInput> {
        self.logical_input.as_ref()
    }
    /// Independently selected authored target schema.
    #[must_use]
    pub fn command(&self) -> &str {
        &self.command
    }
    /// Actual full source availability context.
    #[must_use]
    pub const fn context(&self) -> &ResolvedContext {
        &self.context
    }
    /// Same independently retained invocation dialect.
    #[must_use]
    pub const fn dialect(&self) -> InvocationDialect {
        self.dialect
    }
    /// Source transition lineage, retaining aliases, moves and operand producers.
    #[must_use]
    pub fn lineage(&self) -> &[Arc<OriginalSourceCommandTransition>] {
        &self.lineage
    }
    /// Every applicability limitation remains separate from source syntax.
    #[must_use]
    pub fn obligations(&self) -> &[SourceCommandTransitionObligation] {
        &self.obligations
    }
    /// Complete prior source vectors that left transition applicability unknown.
    /// Their source syntax is retained without asserting that any handler ran.
    #[must_use]
    pub fn uncertain_operations(&self) -> &[Arc<[NativeWord]>] {
        &self.uncertain_operations
    }
    /// Complete original source channel, image and lexer configuration agree.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, config: LexerConfig) -> bool {
        self.site.source.source_image() == image
            && self.config == config
            && self
                .original
                .iter()
                .all(|word| word.image() == image && word.config() == config)
            && self.logical_body.as_ref().is_none_or(|body| {
                body.matches_source(image, config)
                    && self.original.iter().all(|word| {
                        word.span().start() >= body.content_span().start()
                            && word.span().end() <= body.content_span().end()
                    })
            })
            && self.procedure_body.as_ref().is_none_or(|body| {
                body.matches_source(image, config)
                    && self.original.iter().all(|word| {
                        word.span().start() >= body.content_span().start()
                            && word.span().end() <= body.content_span().end()
                    })
            })
            && self.declaration.as_ref().is_none_or(|rows| {
                rows.iter().all(|row| {
                    row.config == config
                        && row.entry.owns_source(&self.site.source, self.site.offset)
                        && crate::registry_invocation::original_native_compiler_words(
                            image,
                            &row.words,
                            self.site.offset,
                            config,
                        )
                        .is_some_and(|words| words.as_slice() == self.original.as_ref())
                })
            })
    }
    /// Actual full availability context and Registry semantic contents agree.
    #[must_use]
    pub fn matches_context(&self, context: &ContextRegistry) -> bool {
        self.context == *context.context()
            && self.registry == context.commands().snapshot().semantic_key()
            && self
                .logical_body
                .as_ref()
                .is_none_or(|body| body.matches_context(context))
            && self
                .procedure_body
                .as_ref()
                .is_none_or(|body| body.matches_context(context))
            && self
                .logical_input
                .as_ref()
                .is_none_or(|input| std::ptr::eq(Arc::as_ptr(&input.context_registry()), context))
    }
    /// Authentic parent Logical source body for this conditional header.
    /// Its geometry supplies no runtime procedure/frame or Native input.
    #[must_use]
    pub fn logical_source_body(
        &self,
    ) -> Option<&crate::registry_invocation::OriginalSourceScriptBody> {
        self.logical_body.as_deref()
    }
    /// Authentic original procedure parent body, without entered execution.
    #[must_use]
    pub fn original_procedure_source_body(
        &self,
    ) -> Option<&crate::registry_invocation::OriginalSourceScriptBody> {
        self.procedure_body.as_deref()
    }
    pub(crate) fn arguments(&self) -> &[SourceAdviceWord] {
        &self.arguments
    }
    pub(crate) fn roles(&self) -> Option<&[(usize, tcl_registry::ArgRole)]> {
        self.roles.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AdviceCell {
    Descriptor(String, Vec<Arc<OriginalSourceCommandTransition>>),
    Declared(Arc<tcl_registry::model::DeclaredCommand>),
    CreatedInterpreter(Box<interpreter_handle::CreatedInterpreterSourceCell>),
    RegisteredInstance(Arc<OriginalSourceRegisteredInstance>),
    SourceClass(Arc<OriginalSourceClassDeclaration>),
    SourceProcedure(Arc<source_procedure::OriginalSourceProcedureBody>),
    ClassInstance(Arc<OriginalSourceClassInstance>),
    Alias {
        target: Box<SourceAdviceNameInput>,
        prefix: Vec<SourceAdviceWord>,
        lookup: AliasTargetLookup,
        lineage: Vec<Arc<OriginalSourceCommandTransition>>,
    },
    Shadowed,
    Deleted,
}
#[derive(Debug, Default)]
pub(crate) struct OriginalSourceTransitionAdviceTape {
    advice: BTreeMap<u32, OriginalSourceCommandTransitionAdvice>,
    declared: BTreeMap<u32, Option<super::DeclaredSourceSelection>>,
    represented: BTreeSet<u32>,
    registry_barriers: BTreeSet<u32>,
    interpreter_bodies: BTreeMap<u32, OriginalSourceInterpreterHandleBody>,
    interpreter_paths: BTreeMap<u32, Vec<OriginalSourceInterpreterPathBinding>>,
    authored_prefixes: BTreeMap<u32, Option<OriginalSourceAuthoredCommandPrefix>>,
    produced_prefixes: BTreeMap<u32, Option<OriginalSourceProducedCommandPrefix>>,
    registered_instances: BTreeMap<u32, Option<OriginalSourceRegisteredInstanceWords>>,
    registered_factories: BTreeMap<u32, Option<Arc<OriginalSourceRegisteredInstance>>>,
    registered_handles: BTreeMap<u32, Option<Arc<OriginalSourceRegisteredHandleBinding>>>,
    class_declarations: BTreeMap<u32, Option<Arc<OriginalSourceClassDeclaration>>>,
    constructor_calls: BTreeMap<u32, Option<OriginalSourceConstructorCall>>,
    class_references: BTreeMap<
        u32,
        Vec<(
            SignatureSourceNameInput,
            Option<OriginalSourceClassReference>,
        )>,
    >,
    configured_classes: BTreeMap<u32, Option<OriginalSourceConfiguredClassReference>>,
    procedure_publications: Option<OriginalSourceProcedurePublications>,
    class_publications: Option<OriginalSourceClassPublications>,
    scanned_procedure_declarations: BTreeSet<u32>,
    class_instance_calls: BTreeMap<u32, Option<OriginalSourceClassInstanceWords>>,
    callback_targets: BTreeMap<
        u32,
        Vec<(
            super::OriginalCallbackPrefix,
            Option<OriginalSourceCallbackProcedureLookup>,
        )>,
    >,
}

/// A represented refusal is authoritative for this conditional source purpose.
/// Only an unrepresented site can consult a separate declaration-source owner.
#[derive(Debug, Clone, Copy)]
pub(crate) enum OriginalSourceTransitionAdviceLookup<'a> {
    Advice(&'a OriginalSourceCommandTransitionAdvice),
    Refused,
    Unrepresented,
}
impl OriginalSourceTransitionAdviceTape {
    pub(crate) fn lookup(&self, offset: u32) -> OriginalSourceTransitionAdviceLookup<'_> {
        if let Some(advice) = self.advice.get(&offset) {
            OriginalSourceTransitionAdviceLookup::Advice(advice)
        } else if self.represented.contains(&offset) {
            OriginalSourceTransitionAdviceLookup::Refused
        } else {
            OriginalSourceTransitionAdviceLookup::Unrepresented
        }
    }

    pub(crate) fn declared_selection(
        &self,
        offset: u32,
    ) -> Option<&super::DeclaredSourceSelection> {
        self.declared.get(&offset)?.as_ref()
    }

    fn retain_declared(&mut self, selection: super::DeclaredSourceSelection) {
        let offset = selection.site().offset;
        self.declared
            .entry(offset)
            .and_modify(|previous| {
                if previous.as_ref() != Some(&selection) {
                    *previous = None;
                }
            })
            .or_insert(Some(selection));
    }

    pub(crate) fn interpreter_body(
        &self,
        offset: u32,
    ) -> Option<&OriginalSourceInterpreterHandleBody> {
        self.interpreter_bodies.get(&offset)
    }

    pub(crate) fn interpreter_paths(
        &self,
        offset: u32,
    ) -> Option<&[OriginalSourceInterpreterPathBinding]> {
        self.interpreter_paths.get(&offset).map(Vec::as_slice)
    }

    pub(crate) fn blocks_registry_source(&self, offset: u32) -> bool {
        self.registry_barriers.contains(&offset)
    }

    fn extend_inventory(&mut self, other: Self) {
        self.scanned_procedure_declarations
            .extend(other.scanned_procedure_declarations);
        for (offset, receipt) in other.class_instance_calls {
            source_class::merge_receipt(&mut self.class_instance_calls, offset, receipt);
        }
        for (offset, targets) in other.callback_targets {
            for (prefix, target) in targets {
                self.merge_callback_target(offset, prefix, target);
            }
        }
        for (offset, references) in other.class_references {
            for (input, receipt) in references {
                self.merge_class_reference(offset, input, receipt);
            }
        }
        for (offset, receipt) in other.configured_classes {
            source_class::merge_receipt(&mut self.configured_classes, offset, receipt);
        }
        self.advice.extend(other.advice);
        self.represented.extend(other.represented);
        self.registry_barriers.extend(other.registry_barriers);
        self.interpreter_bodies.extend(other.interpreter_bodies);
        self.interpreter_paths.extend(other.interpreter_paths);
        self.extend_class_inventory(other.class_declarations, other.constructor_calls);
        for (offset, receipt) in other.registered_factories {
            source_class::merge_receipt(&mut self.registered_factories, offset, receipt);
        }
        for (offset, receipt) in other.registered_instances {
            self.registered_instances
                .entry(offset)
                .and_modify(|previous| {
                    if previous != &receipt {
                        *previous = None;
                    }
                })
                .or_insert(receipt);
        }
        for (offset, receipt) in other.registered_handles {
            self.registered_handles
                .entry(offset)
                .and_modify(|previous| {
                    if previous != &receipt {
                        *previous = None;
                    }
                })
                .or_insert(receipt);
        }
        for (offset, prefix) in other.authored_prefixes {
            self.merge_authored_prefix(offset, prefix);
        }
        for (offset, prefix) in other.produced_prefixes {
            self.merge_produced_prefix(offset, prefix);
        }
        for (offset, selection) in other.declared {
            if let Some(selection) = selection {
                self.retain_declared(selection);
            } else {
                self.declared.insert(offset, None);
            }
        }
    }
}
#[derive(Clone)]
struct AdviceGraph {
    native_baseline: bool,
    cells: HashMap<ByteCommandSlot, AdviceCell>,
    namespaces: BTreeSet<ByteNamespacePath>,
    policy: AdviceNamingPolicy,
    logical_namespace: Option<ByteNamespacePath>,
    dialect: InvocationDialect,
    uncertain_operations: Vec<Arc<[NativeWord]>>,
    procedure_body: Option<Arc<crate::registry_invocation::OriginalSourceScriptBody>>,
    procedure_declarations: Vec<Arc<source_procedure::OriginalSourceProcedureBody>>,
    class_declarations: Vec<Arc<OriginalSourceClassDeclaration>>,
    future_procedure_source: bool,
    procedure_namespace: Option<ByteNamespacePath>,
    procedure_path: Vec<super::CommandAllocationSite>,
    procedure_steps: Arc<std::sync::atomic::AtomicUsize>,
    handles: Vec<(
        registered_instance::SourceHandleKey,
        Arc<OriginalSourceRegisteredHandleBinding>,
    )>,
    class_handles: Vec<(
        registered_instance::SourceHandleKey,
        Arc<OriginalSourceClassHandleBinding>,
    )>,
}
fn root_context(policy: NamePolicyProtocol) -> NativeNameContext<'static> {
    static ROOT: std::sync::LazyLock<ByteNamespacePath> =
        std::sync::LazyLock::new(ByteNamespacePath::root);
    match policy.recipe() {
        NativeNameProtocol::C(_) => NativeNameContext::root(),
        NativeNameProtocol::Jim084 => NativeNameContext::with_jim_namespace(&ROOT, b""),
    }
}
fn lookup_key(policy: &AdviceNamingPolicy, bytes: &[u8]) -> Option<ByteCommandSlot> {
    let Some(policy) = policy.native() else {
        return AdviceNamingPolicy::logical_slot(bytes);
    };
    match policy.recipe() {
        NativeNameProtocol::C(_) => policy
            .recipe()
            .command_lookup_slot(root_context(policy), bytes)
            .ok(),
        NativeNameProtocol::Jim084 => Some(ByteCommandSlot::new(
            ByteNamespacePath::root(),
            NameBytes::from(
                policy
                    .recipe()
                    .command_lookup_input(root_context(policy), bytes)
                    .ok()?
                    .jim_flat_key()?,
            ),
        )),
    }
}
impl AdviceGraph {
    fn new(
        context: &ContextRegistry,
        policy: &AdviceNamingPolicy,
        dialect: InvocationDialect,
    ) -> Option<Self> {
        let mut graph = Self {
            native_baseline: false,
            cells: HashMap::new(),
            namespaces: BTreeSet::from([ByteNamespacePath::root()]),
            policy: policy.clone(),
            logical_namespace: matches!(policy, AdviceNamingPolicy::Logical(_))
                .then(ByteNamespacePath::root),
            dialect,
            uncertain_operations: Vec::new(),
            handles: Vec::new(),
            class_handles: Vec::new(),
            procedure_body: None,
            procedure_declarations: Vec::new(),
            class_declarations: Vec::new(),
            future_procedure_source: false,
            procedure_namespace: None,
            procedure_path: Vec::new(),
            procedure_steps: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        };
        for command in context.commands().command_names() {
            if context
                .context()
                .resolve_spec(context.commands(), command)
                .is_none()
            {
                continue;
            }
            let key = match policy.native().map(NamePolicyProtocol::recipe) {
                Some(NativeNameProtocol::C(_)) => {
                    let native = policy.native()?;
                    native
                        .recipe()
                        .command_c_api_publication_slot(root_context(native), command.as_bytes())
                        .ok()?
                }
                Some(NativeNameProtocol::Jim084) => ByteCommandSlot::new(
                    ByteNamespacePath::root(),
                    NameBytes::from(command.as_bytes()),
                ),
                None => AdviceNamingPolicy::logical_slot(command.as_bytes())?,
            };
            graph.namespaces.insert(key.namespace.clone());
            graph
                .cells
                .insert(key, AdviceCell::Descriptor(command.to_owned(), Vec::new()));
        }
        Some(graph)
    }
    fn native_source_context(&self) -> Option<NativeNameContext<'_>> {
        let policy = self.policy.native()?;
        self.procedure_namespace.as_ref().map_or_else(
            || Some(root_context(policy)),
            |namespace| Some(NativeNameContext::new(namespace)),
        )
    }
    fn lookup_head_key(&self, bytes: &[u8]) -> Option<ByteCommandSlot> {
        if let Some(namespace) = &self.procedure_namespace {
            let native = self.policy.native()?;
            let local = native
                .recipe()
                .command_lookup_slot(NativeNameContext::new(namespace), bytes)
                .ok()?;
            return if self.cells.contains_key(&local) {
                Some(local)
            } else {
                lookup_key(&self.policy, bytes)
            };
        }
        let Some(namespace) = &self.logical_namespace else {
            return lookup_key(&self.policy, bytes);
        };
        let candidates =
            tcl_syntax::naming::authored_source_command_candidates(namespace, &[], bytes)?;
        candidates
            .iter()
            .find(|key| self.cells.contains_key(*key))
            .cloned()
            .or_else(|| candidates.last().cloned())
    }

    fn publication_key(
        &self,
        bytes: &[u8],
        purpose: AdvicePublicationPurpose,
    ) -> Option<ByteCommandSlot> {
        if let Some(namespace) = &self.procedure_namespace {
            let recipe = self.policy.native()?.recipe();
            let context = NativeNameContext::new(namespace);
            return match purpose {
                AdvicePublicationPurpose::Define => recipe.command_publication_slot(context, bytes),
                AdvicePublicationPurpose::Move => recipe.rename_destination_slot(context, bytes),
                AdvicePublicationPurpose::Alias => recipe.alias_publication_slot(context, bytes),
            }
            .ok();
        }
        self.logical_namespace.as_ref().map_or_else(
            || self.policy.publication_slot(bytes, purpose),
            |namespace| tcl_syntax::naming::authored_source_command_slot(namespace, bytes),
        )
    }

    fn alias_target_key(
        &self,
        target: &SourceAdviceNameInput,
        lookup: AliasTargetLookup,
    ) -> Option<ByteCommandSlot> {
        match lookup {
            AliasTargetLookup::Global => lookup_key(&self.policy, target.bytes()),
            AliasTargetLookup::CallerNamespace => self.lookup_head_key(target.bytes()),
        }
    }

    fn declared_source_selection(
        &self,
        origin: &Arc<super::SourceOriginId>,
        head: &SourceAdviceNameInput,
        words: &[NativeWord],
    ) -> Option<super::DeclaredSourceSelection> {
        // naming.source.original-declared-command-word-contract
        // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
        let key = self.lookup_head_key(head.bytes())?;
        let AdviceCell::Declared(declaration) = self.cells.get(&key)? else {
            return None;
        };
        super::DeclaredSourceSelection::authored(
            super::CommandAllocationSite {
                source: Arc::clone(origin),
                offset: words.first()?.span().start(),
            },
            words,
            head.clone(),
            Arc::clone(declaration),
            !self.uncertain_operations.is_empty(),
        )
    }
    fn record_uncertainty(&mut self, original: &[NativeWord]) {
        if !self
            .uncertain_operations
            .iter()
            .any(|operation| operation.as_ref() == original)
        {
            self.uncertain_operations.push(Arc::from(original));
        }
    }
    fn widen(&mut self, original: &[NativeWord]) {
        self.uncertain_operations.push(Arc::from(original));
        self.handles.clear();
        self.cells.retain(|_, cell| match cell {
            AdviceCell::Descriptor(_, lineage) => lineage.is_empty(),
            AdviceCell::Shadowed
            | AdviceCell::Deleted
            | AdviceCell::Declared(_)
            | AdviceCell::SourceProcedure(_)
            | AdviceCell::SourceClass(_)
            | AdviceCell::ClassInstance(_)
            | AdviceCell::RegisteredInstance(_)
            | AdviceCell::Alias { .. } => true,
            AdviceCell::CreatedInterpreter(_) => false,
        });
    }
    fn blocks_registry_source(&self, input: &SourceAdviceNameInput) -> bool {
        let Some(start) = self.lookup_head_key(input.bytes()) else {
            return false;
        };
        let mut blocked = false;
        let cycle = tcl_syntax::naming::alias_chain_loops(start, |key| match self.cells.get(key) {
            Some(AdviceCell::Alias { target, lookup, .. }) => {
                Ok::<_, ()>(self.alias_target_key(target, *lookup))
            }
            Some(
                AdviceCell::Shadowed
                | AdviceCell::Deleted
                | AdviceCell::CreatedInterpreter(_)
                | AdviceCell::RegisteredInstance(_)
                | AdviceCell::SourceClass(_)
                | AdviceCell::ClassInstance(_)
                | AdviceCell::SourceProcedure(_)
                | AdviceCell::Declared(_),
            ) => {
                blocked = true;
                Ok(None)
            }
            Some(AdviceCell::Descriptor(_, _)) | None => Ok(None),
        });
        blocked || cycle == Ok(true)
    }

    fn resolve(
        &self,
        input: &SourceAdviceNameInput,
    ) -> Option<(
        String,
        Vec<SourceAdviceWord>,
        Vec<Arc<OriginalSourceCommandTransition>>,
    )> {
        let start = self.lookup_head_key(input.bytes())?;
        if tcl_syntax::naming::alias_chain_loops(start.clone(), |key| match self.cells.get(key) {
            Some(AdviceCell::Alias { target, lookup, .. }) => {
                Ok::<_, ()>(Some(self.alias_target_key(target, *lookup).ok_or(())?))
            }
            Some(AdviceCell::Descriptor(_, _)) | None => Ok(None),
            Some(
                AdviceCell::Shadowed
                | AdviceCell::Deleted
                | AdviceCell::CreatedInterpreter(_)
                | AdviceCell::RegisteredInstance(_)
                | AdviceCell::SourceClass(_)
                | AdviceCell::ClassInstance(_)
                | AdviceCell::SourceProcedure(_)
                | AdviceCell::Declared(_),
            ) => Err(()),
        })
        .ok()?
        {
            return None;
        }
        let mut key = start;
        let mut prefix = Vec::new();
        let mut lineage = Vec::new();
        loop {
            match self.cells.get(&key)? {
                AdviceCell::Descriptor(command, steps) => {
                    lineage.extend(steps.iter().cloned());
                    return Some((command.clone(), prefix, lineage));
                }
                AdviceCell::Alias {
                    target,
                    prefix: captured,
                    lookup,
                    lineage: steps,
                } => {
                    let mut next = captured.clone();
                    next.extend(prefix);
                    prefix = next;
                    lineage.extend(steps.iter().cloned());
                    key = self.alias_target_key(target, *lookup)?;
                }
                AdviceCell::Shadowed
                | AdviceCell::Deleted
                | AdviceCell::CreatedInterpreter(_)
                | AdviceCell::RegisteredInstance(_)
                | AdviceCell::SourceClass(_)
                | AdviceCell::ClassInstance(_)
                | AdviceCell::SourceProcedure(_)
                | AdviceCell::Declared(_) => {
                    return None;
                }
            }
        }
    }
    fn apply(&mut self, receipt: &Arc<OriginalSourceCommandTransition>) -> Option<()> {
        for fact in receipt.transitions.facts() {
            match &fact.transition {
                StateTransition::CommandBinding(transition) => {
                    self.apply_command(receipt, transition)?;
                }
                StateTransition::Interpreter(transition) => {
                    self.apply_interpreter_transition(receipt, transition)?;
                }
                StateTransition::Namespace(tcl_registry::NamespaceTransition::Ensure {
                    namespace: tcl_registry::NamespaceTransitionTarget::Named(subject),
                }) => {
                    receipt.input(subject)?;
                    if !matches!(
                        self.policy.native().map(NamePolicyProtocol::recipe),
                        Some(NativeNameProtocol::Jim084)
                    ) {
                        let mut path = self.logical_namespace.as_ref().map_or_else(
                            || self.policy.namespace_path(receipt.selected_input(subject)?),
                            |namespace| {
                                tcl_syntax::naming::authored_source_namespace_path(
                                    namespace,
                                    receipt.selected_input(subject)?,
                                )
                            },
                        )?;
                        loop {
                            self.namespaces.insert(path.clone());
                            let Some(parent) = path.parent() else {
                                break;
                            };
                            path = parent;
                        }
                    }
                }
                StateTransition::Namespace(_) => return None,
                _ => {}
            }
        }
        Some(())
    }
    fn apply_move(
        &mut self,
        receipt: &Arc<OriginalSourceCommandTransition>,
        from: &TransitionSubject,
        to: &TransitionSubject,
    ) -> Option<()> {
        let from = self.policy.rename_source(receipt.input(from)?.bytes())?;
        let from_key = self.lookup_head_key(&from)?;
        let cell = self.cells.get(&from_key)?.clone();
        let destination = receipt.input(to)?;
        let selected = self.policy.rename_destination(destination.bytes())?;
        if matches!(cell, AdviceCell::Shadowed | AdviceCell::Deleted) {
            return None;
        }
        if selected.is_empty() {
            self.cells.insert(from_key, AdviceCell::Deleted);
            return Some(());
        }
        let to_key = self.publication_key(destination.bytes(), AdvicePublicationPurpose::Move)?;
        if !self.namespaces.contains(&to_key.namespace) {
            return None;
        }
        if self
            .cells
            .get(&to_key)
            .is_some_and(|cell| !matches!(cell, AdviceCell::Deleted))
        {
            return None;
        }
        self.cells.insert(from_key, AdviceCell::Deleted);
        let moved = match cell {
            AdviceCell::Descriptor(command, mut lineage) => {
                lineage.push(Arc::clone(receipt));
                AdviceCell::Descriptor(command, lineage)
            }
            AdviceCell::Alias {
                target,
                prefix,
                lookup,
                mut lineage,
            } => {
                lineage.push(Arc::clone(receipt));
                AdviceCell::Alias {
                    target,
                    prefix,
                    lookup,
                    lineage,
                }
            }
            AdviceCell::CreatedInterpreter(cell) => {
                AdviceCell::CreatedInterpreter(cell.moved(to_key.clone(), receipt))
            }
            AdviceCell::RegisteredInstance(instance) => {
                AdviceCell::RegisteredInstance(Arc::new(instance.moved(to_key.clone(), receipt)))
            }
            AdviceCell::ClassInstance(instance) => {
                AdviceCell::ClassInstance(Arc::new(instance.moved(to_key.clone(), receipt)))
            }
            AdviceCell::SourceClass(class) => {
                AdviceCell::SourceClass(Arc::new(class.moved(to_key.clone(), receipt)))
            }
            AdviceCell::SourceProcedure(procedure) => {
                AdviceCell::SourceProcedure(Arc::new(procedure.moved(to_key.clone(), receipt)))
            }
            AdviceCell::Declared(_) => AdviceCell::Shadowed,
            AdviceCell::Shadowed | AdviceCell::Deleted => return None,
        };
        self.cells.insert(to_key, moved);
        Some(())
    }

    fn apply_command(
        &mut self,
        receipt: &Arc<OriginalSourceCommandTransition>,
        transition: &CommandBindingTransition,
    ) -> Option<()> {
        let input = |subject| receipt.input(subject);
        match transition {
            CommandBindingTransition::Define { name, .. } => {
                let key =
                    self.publication_key(input(name)?.bytes(), AdvicePublicationPurpose::Define)?;
                self.cells.insert(key, AdviceCell::Shadowed);
            }
            CommandBindingTransition::Move { from, to } => {
                self.apply_move(receipt, from, to)?;
            }
            CommandBindingTransition::Delete { interpreter, name } => {
                if interpreter
                    .as_ref()
                    .is_some_and(|subject| receipt.is_current_interpreter(subject) != Some(true))
                {
                    return None;
                }
                self.cells.insert(
                    self.lookup_head_key(input(name)?.bytes())?,
                    AdviceCell::Deleted,
                );
            }
            CommandBindingTransition::Alias {
                source_interpreter,
                alias,
                target_interpreter,
                target,
                arguments,
                target_lookup,
            } => {
                if !receipt.is_current_interpreter(source_interpreter)? {
                    return None;
                }
                let alias = input(alias)?;
                let key = self.publication_key(alias.bytes(), AdvicePublicationPurpose::Alias)?;
                if !self.namespaces.contains(&key.namespace)
                    || !receipt.is_current_interpreter(target_interpreter)?
                {
                    // A current alias with foreign target semantics withdraws
                    // any earlier current procedure/class meaning at its name.
                    self.cells.insert(key, AdviceCell::Shadowed);
                    return Some(());
                }
                let prefix = arguments
                    .iter()
                    .enumerate()
                    .map(|(ordinal, subject)| {
                        let input = input(subject)?.clone();
                        let original = input.original_word()?.clone();
                        Some(SourceAdviceWord {
                            original,
                            value: Some(Arc::from(input.bytes())),
                            input: Some(input),
                            origin: crate::registry_invocation::InvocationWordOrigin::BindingPrefix(
                                ordinal,
                            ),
                        })
                    })
                    .collect::<Option<Vec<_>>>()?;
                self.cells.insert(
                    key,
                    AdviceCell::Alias {
                        target: Box::new(input(target)?.clone()),
                        prefix,
                        lookup: *target_lookup,
                        lineage: vec![Arc::clone(receipt)],
                    },
                );
            }
            CommandBindingTransition::Unknown { .. } => return None,
        }
        Some(())
    }
}

fn original_words(
    native: &[NativeWord],
    policy: &AdviceNamingPolicy,
) -> Option<Vec<SourceAdviceWord>> {
    if matches!(policy, AdviceNamingPolicy::Logical(_)) {
        return Some(
            native
                .iter()
                .enumerate()
                .map(|(ordinal, original)| {
                    let value =
                        tcl_syntax::word_rules::original_static_word_unicode_value(original)
                            .filter(|bytes| {
                                std::str::from_utf8(bytes).is_ok() && !bytes.contains(&0)
                            })
                            .map(Arc::<[u8]>::from);
                    SourceAdviceWord {
                        original: original.clone(),
                        input: value.as_ref().map(|bytes| {
                            SourceAdviceNameInput::Logical(OriginalLogicalSourceNameInput {
                                original: original.clone(),
                                bytes: Arc::clone(bytes),
                            })
                        }),
                        value,
                        origin: crate::registry_invocation::InvocationWordOrigin::Written(ordinal),
                    }
                })
                .collect(),
        );
    }
    let policy = policy.native()?;
    let captured = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
        native,
        policy.string_protocol(),
    )
    .ok()?;
    Some(
        native
            .iter()
            .enumerate()
            .map(|(ordinal, original)| SourceAdviceWord {
                original: original.clone(),
                input: SignatureSourceNameKey::from_original_native_word(
                    original,
                    tcl_syntax::word_rules::WordValueRules::from_config(&original.config()),
                    policy,
                )
                .map(SignatureSourceNameInput::OriginalWord)
                .map(SourceAdviceNameInput::Native),
                value: captured.literal(ordinal).map(Arc::from),
                origin: crate::registry_invocation::InvocationWordOrigin::Written(ordinal),
            })
            .collect(),
    )
}
fn registry_words(arguments: &[SourceAdviceWord]) -> Vec<tcl_registry::InvocationWord<'_>> {
    arguments
        .iter()
        .map(|argument| {
            if argument.original.group().expand {
                return tcl_registry::InvocationWord::Expanded;
            }
            argument
                .value
                .as_deref()
                .map_or(tcl_registry::InvocationWord::Dynamic, |bytes| {
                    crate::registry_invocation::source_structure::source_schema_word(
                        tcl_registry::InvocationWord::KnownBytes(bytes),
                    )
                })
        })
        .collect()
}
fn transition_subjects(transitions: &tcl_registry::StateTransitions) -> Vec<TransitionSubject> {
    let mut subjects = Vec::new();
    for fact in transitions.facts() {
        match &fact.transition {
            StateTransition::CommandBinding(CommandBindingTransition::Define { name, .. }) => {
                subjects.push(name.clone());
            }
            StateTransition::CommandBinding(CommandBindingTransition::Move { from, to }) => {
                subjects.extend([from.clone(), to.clone()]);
            }
            StateTransition::CommandBinding(CommandBindingTransition::Delete {
                interpreter,
                name,
            }) => {
                subjects.extend(interpreter.iter().cloned());
                subjects.push(name.clone());
            }
            StateTransition::CommandBinding(CommandBindingTransition::Alias {
                source_interpreter,
                alias,
                target_interpreter,
                target,
                arguments,
                ..
            }) => {
                subjects.extend([
                    source_interpreter.clone(),
                    alias.clone(),
                    target_interpreter.clone(),
                    target.clone(),
                ]);
                subjects.extend(arguments.iter().cloned());
            }
            StateTransition::Interpreter(
                tcl_registry::InterpreterTransition::Create {
                    interpreter: Some(subject),
                    ..
                }
                | tcl_registry::InterpreterTransition::Delete {
                    interpreter: subject,
                },
            )
            | StateTransition::Namespace(tcl_registry::NamespaceTransition::Ensure {
                namespace: tcl_registry::NamespaceTransitionTarget::Named(subject),
            }) => subjects.push(subject.clone()),
            _ => {}
        }
    }
    subjects
}
/// A qualifier selected by the Registry retains the complete original Move
/// operand. It never becomes a fabricated original namespace word.
fn original_transition_subject_matches(
    subject: &TransitionSubject,
    input: &SourceAdviceNameInput,
    transitions: &tcl_registry::StateTransitions,
) -> bool {
    let Some(selected) = subject
        .native_bytes()
        .or_else(|| subject.literal().map(str::as_bytes))
    else {
        return false;
    };
    if selected == input.bytes() {
        return true;
    }
    let is_qualifier = transitions.facts().iter().any(|fact| {
        matches!(&fact.transition,
        StateTransition::Namespace(tcl_registry::NamespaceTransition::Ensure {
            namespace: tcl_registry::NamespaceTransitionTarget::Named(owned),
        }) if owned == subject)
    });
    if !is_qualifier {
        return false;
    }
    transitions.command_bindings().any(|transition| {
        let CommandBindingTransition::Move { to, .. } = transition else {
            return false;
        };
        if to.argument_index() != subject.argument_index()
            || to
                .native_bytes()
                .or_else(|| to.literal().map(str::as_bytes))
                != Some(input.bytes())
        {
            return false;
        }
        let destination = match input {
            SourceAdviceNameInput::Native(original) => original
                .policy()
                .recipe()
                .rename_destination_input(root_context(original.policy()), original.bytes())
                .ok()
                .map(|projection| projection.selected().to_vec()),
            SourceAdviceNameInput::Logical(original) => Some(original.bytes().to_vec()),
        };
        destination.is_some_and(|bytes| {
            tcl_registry::state_transition::namespace_qualifiers_bytes(&bytes) == selected
        })
    })
}
fn capture_transition(
    origin: &Arc<super::SourceOriginId>,
    native: &[NativeWord],
    command: &str,
    arguments: &[SourceAdviceWord],
    transitions: tcl_registry::StateTransitions,
    selection_lineage: &[Arc<OriginalSourceCommandTransition>],
) -> Option<Arc<OriginalSourceCommandTransition>> {
    let inputs = transition_subjects(&transitions)
        .into_iter()
        .map(|subject| {
            let Some(ordinal) = subject.argument_index() else {
                // Only the schema-owned current-interpreter constant is
                // supported without an operand producer. Its provenance stays
                // in the retained transition recipe, never in source inputs.
                return matches!(&subject, TransitionSubject::Literal(value) if value.is_empty())
                    .then_some(None);
            };
            let input = arguments.get(ordinal)?.input.as_ref()?.clone();
            original_transition_subject_matches(&subject, &input, &transitions)
                .then_some(Some((subject, input)))
        })
        .collect::<Option<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();
    Some(Arc::new(OriginalSourceCommandTransition {
        site: super::CommandAllocationSite {
            source: Arc::clone(origin),
            offset: native.first()?.span().start(),
        },
        original: Arc::from(native),
        inputs,
        descriptor: command.to_owned(),
        transitions,
        selection_lineage: selection_lineage.to_vec(),
    }))
}

struct AdviceTapeInputs<'a> {
    origin: &'a Arc<super::SourceOriginId>,
    config: LexerConfig,
    policy: AdviceNamingPolicy,
    dialect: InvocationDialect,
    plan: tcl_lexer::NativeScriptWordsPlan,
}

struct AdviceInvocationContext<'a> {
    origin: &'a Arc<super::SourceOriginId>,
    context: &'a ContextRegistry,
    config: LexerConfig,
    dialect: InvocationDialect,
    policy: &'a AdviceNamingPolicy,
}
#[derive(Clone, Copy)]
struct AdviceInvocation<'a> {
    native: &'a [NativeWord],
    head: &'a SourceAdviceNameInput,
    arguments: &'a [SourceAdviceWord],
    lineage: &'a [Arc<OriginalSourceCommandTransition>],
}

impl AdviceInvocationContext<'_> {
    fn apply_schema_transition(
        &self,
        graph: &mut AdviceGraph,
        invocation: AdviceInvocation<'_>,
        command: &str,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> Option<OriginalSourceTransitionAdviceTape> {
        let AdviceInvocation {
            native,
            arguments,
            lineage,
            ..
        } = invocation;
        let mut inventory = OriginalSourceTransitionAdviceTape::default();
        let transitions = schema.state_transitions();
        let receipt = capture_transition(
            self.origin,
            native,
            command,
            arguments,
            transitions.clone(),
            lineage,
        );
        if let Some(receipt) = &receipt {
            let applied = graph.apply(receipt).is_some();
            if !applied {
                graph.widen(native);
            }
        }
        let declared = schema.semantics.state_transitions.is_declared();
        let immediate_body = schema
            .authored_source_argument_roles()
            .0
            .iter()
            .any(|(_, role)| *role == tcl_registry::ArgRole::Body)
            && !schema
                .semantics
                .traits
                .contains(tcl_registry::Traits::DEFERS_BODY);
        if (receipt.is_none() && !transitions.facts().is_empty())
            || transitions.widens(tcl_registry::StateTransitionDomain::CommandBindings)
            || transitions.widens(tcl_registry::StateTransitionDomain::CommandResolution)
        {
            graph.widen(native);
        } else if immediate_body {
            let advice = self.schema_advice(invocation, graph, schema)?;
            let projected = self.apply_immediate_body_effects(graph, &advice, schema);
            if !projected.effects_complete {
                graph.widen(native);
            }
            inventory.extend_inventory(projected.inventory);
        } else if !declared {
            graph.widen(native);
        }
        Some(inventory)
    }

    fn retain_source_inventory(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        invocation: AdviceInvocation<'_>,
        graph: &AdviceGraph,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> Option<()> {
        self.retain_schema_advice(tape, invocation, graph, schema)?;
        self.retain_authored_command_prefix(tape, invocation, graph, schema, None);
        self.retain_source_callback_targets(tape, invocation, graph, schema);
        self.retain_produced_command_prefixes(tape, invocation, graph, schema, None);
        self.retain_logical_procedure_body(tape, invocation, graph, schema);
        Some(())
    }

    fn retain_schema_advice(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        invocation: AdviceInvocation<'_>,
        graph: &AdviceGraph,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> Option<()> {
        if graph.native_baseline
            || !invocation.lineage.is_empty()
            || graph.procedure_body.is_some()
            || matches!(self.policy, AdviceNamingPolicy::Logical(_))
        {
            let advice = self.schema_advice(invocation, graph, schema)?;
            tape.advice.insert(advice.site.offset, advice);
        }
        Some(())
    }

    fn authored_roles(
        &self,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> (Vec<(u8, tcl_registry::ArgRole)>, bool) {
        match self.policy {
            AdviceNamingPolicy::Logical(_) => schema.authored_logical_source_argument_roles(),
            AdviceNamingPolicy::Native(_) => schema.authored_source_argument_roles(),
        }
    }

    fn schema_advice(
        &self,
        invocation: AdviceInvocation<'_>,
        graph: &AdviceGraph,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> Option<OriginalSourceCommandTransitionAdvice> {
        let AdviceInvocation {
            native,
            head,
            arguments,
            lineage,
        } = invocation;
        let facts = schema.facts();
        let (roles, complete) = self.authored_roles(schema);
        let roles = complete
            .then(|| {
                roles
                    .into_iter()
                    .map(|(ordinal, role)| {
                        facts
                            .argument_offset
                            .checked_add(usize::from(ordinal))
                            .map(|ordinal| (ordinal, role))
                    })
                    .collect::<Option<Vec<_>>>()
            })
            .flatten();
        let mut obligations = vec![SourceCommandTransitionObligation::UnavailableActualLookup];
        if graph.native_baseline {
            obligations.push(SourceCommandTransitionObligation::NativeBaselineSourceApplicability);
        }
        if !lineage.is_empty() {
            obligations.push(SourceCommandTransitionObligation::WrittenTransitionApplicability);
        }
        if graph.procedure_body.is_some() {
            obligations.push(SourceCommandTransitionObligation::OriginalProcedureBodyApplicability);
        }
        if graph.future_procedure_source {
            obligations.push(
                SourceCommandTransitionObligation::FutureOriginalProcedureSourceApplicability,
            );
        }
        if matches!(self.policy, AdviceNamingPolicy::Logical(_)) {
            obligations.push(SourceCommandTransitionObligation::ExplicitLogicalSourceApplicability);
        }
        if !graph.uncertain_operations.is_empty() {
            obligations.push(SourceCommandTransitionObligation::UnknownEarlierMutation);
        }
        Some(OriginalSourceCommandTransitionAdvice {
            site: super::CommandAllocationSite {
                source: Arc::clone(self.origin),
                offset: native.first()?.span().start(),
            },
            original: Arc::from(native),
            head: head.clone(),
            arguments: arguments.to_vec(),
            command: facts.canonical_command.clone(),
            context: self.context.context().clone(),
            registry: self.context.commands().snapshot().semantic_key(),
            config: self.config,
            dialect: self.dialect,
            roles,
            lineage: lineage.to_vec(),
            obligations,
            uncertain_operations: graph.uncertain_operations.clone(),
            declaration: None,
            logical_body: None,
            procedure_body: graph.procedure_body.clone(),
            logical_input: match self.policy {
                AdviceNamingPolicy::Logical(input) => Some(input.clone()),
                AdviceNamingPolicy::Native(_) => None,
            },
        })
    }
}

impl AdviceInvocationContext<'_> {
    fn retain_source_header_relations(
        &self,
        graph: &mut AdviceGraph,
        tape: &mut OriginalSourceTransitionAdviceTape,
        native: &[NativeWord],
        head: &SourceAdviceNameInput,
    ) -> bool {
        if let Some(declared) = graph.declared_source_selection(self.origin, head, native) {
            tape.retain_declared(declared);
        }
        if let Some(body) = self.interpreter_handle_body(graph, head, native) {
            tape.interpreter_bodies
                .insert(body.invocation_offset(), body);
            // The child can call its parent; no later parent map follows.
            graph.widen(native);
            return true;
        }
        false
    }

    fn retain_original_invocation(
        &self,
        graph: &mut AdviceGraph,
        tape: &mut OriginalSourceTransitionAdviceTape,
        invocation: &tcl_lexer::NativeScriptCommandWords,
    ) -> Option<()> {
        tape.represented
            .insert(invocation.words.first()?.span().start());
        let Some(written) = original_words(&invocation.words, self.policy) else {
            graph.widen(&invocation.words);
            return Some(());
        };
        self.retain_original_operand_constructor_calls(tape, graph, &invocation.words, None);
        if self.retain_original_source_receiver(tape, graph, &invocation.words, &written) {
            return Some(());
        }
        let handle = self.before_operand_handle_binding(graph, &invocation.words, &written);
        let class_handle =
            self.before_operand_class_handle(tape, graph, &invocation.words, &written);
        let Some(head) = written
            .first()
            .and_then(|head| head.input.as_ref())
            .cloned()
        else {
            if matches!(self.policy, AdviceNamingPolicy::Logical(_)) {
                // The original children execute before an unknown parent head;
                // retaining them does not select that head from result data.
                let _ = self.inspect_operand_effects(graph, &invocation.words, tape);
                tape.registry_barriers
                    .insert(invocation.words.first()?.span().start());
            }
            graph.widen(&invocation.words);
            return Some(());
        };
        let original_selection = (graph.native_baseline || class_handle.is_some())
            .then(|| graph.resolve(&head))
            .flatten();
        let original_barrier = graph.blocks_registry_source(&head);
        let Some(operand_barrier) = self.inspect_operand_effects(graph, &invocation.words, tape)
        else {
            graph.widen(&invocation.words);
            return Some(());
        };
        if original_barrier {
            tape.registry_barriers
                .insert(invocation.words.first()?.span().start());
        }
        if self.retain_source_header_relations(graph, tape, &invocation.words, &head) {
            return Some(());
        }
        let selected = original_selection.or_else(|| graph.resolve(&head));
        let Some((command, mut arguments, lineage)) = selected else {
            graph.widen(&invocation.words);
            return Some(());
        };
        // Prefix ordinals belong to the composed effective prefix. Each
        // retained word/input still owns its original transition producer.
        for (ordinal, argument) in arguments.iter_mut().enumerate() {
            argument.origin =
                crate::registry_invocation::InvocationWordOrigin::BindingPrefix(ordinal);
        }
        arguments.extend(written.into_iter().skip(1));
        let words = registry_words(&arguments);
        let resolution =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                self.context.commands(),
                Some(self.context.context()),
                tcl_registry::InvocationWords::structured(
                    tcl_registry::InvocationWord::Literal(&command),
                    &words,
                )
                .with_dialect(self.dialect),
                tcl_dialect::model::InvocationRealm::RuleLoader,
            );
        let Some(schema) = resolution.resolved() else {
            graph.widen(&invocation.words);
            return Some(());
        };
        let selected = AdviceInvocation {
            native: &invocation.words,
            head: &head,
            arguments: &arguments,
            lineage: &lineage,
        };
        self.retain_source_inventory(tape, selected, graph, &schema)?;
        self.retain_interpreter_paths(tape, selected, graph, &schema);
        let factory = self.registered_factory(graph, selected, &schema);
        let class = self.source_class_declaration(graph, selected, &schema);
        let procedure = self.source_procedure_declaration(graph, selected, &schema);
        self.retain_source_configured_class(tape, graph, selected, &schema);
        self.retain_source_class_operand_references(
            tape,
            graph,
            class.as_ref(),
            procedure.as_ref(),
        );
        let body_inventory = self.apply_schema_transition(graph, selected, &command, &schema)?;
        tape.extend_inventory(body_inventory);
        if operand_barrier {
            // The current receipt is only a pre-operand source possibility.
            // Unknown operand callbacks cannot preserve the later graph.
            graph.widen(&invocation.words);
        }
        self.install_registered_results(tape, graph, factory, handle);
        Self::install_source_class_handle(graph, class_handle);
        self.retain_source_class_adoptions(tape, graph, class.as_ref());
        Self::install_source_class(tape, graph, class);
        source_procedure::install_original_procedure(graph, procedure);
        Some(())
    }
}

impl super::SourceCommandBindings {
    pub(crate) const fn original_source_lexer_config(&self) -> Option<LexerConfig> {
        self.lexer_config
    }

    pub(crate) fn original_logical_source_name_advice_input(
        &self,
    ) -> Option<&crate::analyser::ResolvedAnalysisInput> {
        self.final_state.logical_source_name_advice_input()
    }

    fn prepare_source_advice_tape(
        &self,
        context: &ContextRegistry,
    ) -> Option<AdviceTapeInputs<'_>> {
        let origin = self.source_origin()?;
        let config = self.lexer_config?;
        let baseline = &self.final_state.baseline;
        let policy = if let Some(native) = baseline
            .execution_name_policy
            .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
        {
            AdviceNamingPolicy::Native(native)
        } else {
            AdviceNamingPolicy::Logical(
                self.final_state.logical_source_name_advice_input()?.clone(),
            )
        };
        let dialect = baseline.dialect?;
        if baseline.registry_snapshot.as_ref()
            != Some(&context.commands().snapshot().semantic_key())
            || !self.matches_original_source_image(origin.source_image(), config)
        {
            return None;
        }
        let plan = tcl_lexer::native_script_words_in(
            origin.source_image().clone(),
            tcl_lexer::Span::new(0, u32::try_from(origin.source_image().bytes().len()).ok()?),
            config,
        )
        .ok()?;
        if plan.fatal_tail.is_some() {
            return None;
        }
        Some(AdviceTapeInputs {
            origin,
            config,
            policy,
            dialect,
            plan,
        })
    }

    fn initial_source_advice_graph(
        &self,
        context: &ContextRegistry,
        policy: &AdviceNamingPolicy,
    ) -> Option<AdviceGraph> {
        let baseline = &self.final_state.baseline;
        let mut graph = AdviceGraph::new(context, policy, baseline.dialect?)?;
        if let Some(entry) = &baseline.native_entry {
            graph.retain_native_baseline(context, entry)?;
        }
        for command in baseline.declared_commands.keys() {
            if let Some(key) = lookup_key(policy, command.as_bytes()) {
                let surface = baseline
                    .declared_surface
                    .as_ref()?
                    .document_surface(context.commands());
                let name = baseline.declared_commands.get(command)?;
                let declaration = surface.declared_command(name)?;
                graph
                    .cells
                    .insert(key, AdviceCell::Declared(Arc::new(declaration.clone())));
            }
        }
        Some(graph)
    }

    /// Independently interpreted authored root-script transitions. This owner
    /// never mutates, publishes or borrows the retained runtime command state.
    pub(crate) fn original_source_transition_advice_tape(
        &self,
        context: &ContextRegistry,
    ) -> Option<OriginalSourceTransitionAdviceTape> {
        let AdviceTapeInputs {
            origin,
            config,
            policy,
            dialect,
            plan,
        } = self.prepare_source_advice_tape(context)?;
        let mut graph = self.initial_source_advice_graph(context, &policy)?;
        let mut tape = OriginalSourceTransitionAdviceTape::default();
        let advice_context = AdviceInvocationContext {
            origin,
            context,
            config,
            dialect,
            policy: &policy,
        };
        for invocation in plan.commands {
            advice_context.retain_original_invocation(&mut graph, &mut tape, &invocation)?;
        }
        advice_context.retain_future_original_procedure_bodies(&mut tape, &graph);
        advice_context.retain_final_original_procedure_publications(&mut tape, &graph);
        advice_context.retain_final_original_class_publications(&mut tape, &graph);
        Some(tape)
    }
}

#[cfg(test)]
mod tests {
    use crate::analyser::Analyser;
    use crate::registry_invocation::InvocationWordOrigin;
    use crate::registry_invocation::source_structure::{
        OriginalRegistrySource, source_registry_words,
    };

    #[test]
    fn logical_original_advice_words_keep_lossless_values_and_native_units_independent() {
        // naming.source.authored-command-transition-advice
        // docs/design/analysis/name-resolution-proofs/authored-command-transition-advice.md
        use super::{AdviceNamingPolicy, original_words};
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let context = tcl_registry::model::ingress::context_for_profile(profile);
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(profile, profile, context, config);
        assert!(input.has_logical_source_name_context());
        let source = r"rename café\uFFFD café\uD800";
        let plan = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::document(source),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        let originals = &plan.commands[0].words;
        let words = original_words(originals, &AdviceNamingPolicy::Logical(input)).unwrap();
        assert_eq!(words[1].value.as_deref(), Some("café�".as_bytes()));
        assert!(words[1].input.as_ref().unwrap().logical_input().is_some());
        assert!(words[1].input.as_ref().unwrap().native_input().is_none());
        assert_eq!(words[1].original, originals[1]);
        assert!(words[2].value.is_none());
        assert!(words[2].input.is_none());
        assert_eq!(words[2].original, originals[2]);
        assert_eq!(
            tcl_syntax::word_rules::original_static_word_source_bytes(&originals[2]).as_deref(),
            Some("café�".as_bytes())
        );

        let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        let native_plan = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::document(source),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
        )
        .unwrap();
        let native = original_words(
            &native_plan.commands[0].words,
            &AdviceNamingPolicy::Native(dialect.authored_name_policy().unwrap()),
        )
        .unwrap();
        assert!(native[2].value.is_some());
        assert!(native[2].input.as_ref().unwrap().native_input().is_some());
        assert!(native[2].input.as_ref().unwrap().logical_input().is_none());
    }

    #[test]
    fn original_move_qualifier_retains_the_complete_operand_and_rejects_other_projections() {
        // naming.interpreter.original-created-handle-source-body
        // docs/design/analysis/name-resolution-proofs/interpreter-original-created-handle-source-body.md
        // This is a source-projection guard, not a successful runtime rename.
        use super::{AdviceNamingPolicy, original_transition_subject_matches, original_words};
        use tcl_registry::{
            CommandBindingTransition, NamespaceTransition, NamespaceTransitionTarget,
            StateTransition, StateTransitions, TransitionSubject,
        };
        let source = "rename s ::N::held";
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = tcl_registry::InvocationDialect::for_version(version);
            let policy = dialect.authored_name_policy().unwrap();
            let plan = tcl_lexer::native_script_words_in(
                tcl_lexer::SourceImage::document(source),
                tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
                tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
            )
            .unwrap();
            let words =
                original_words(&plan.commands[0].words, &AdviceNamingPolicy::Native(policy))
                    .unwrap();
            let input = words[2].input.as_ref().unwrap();
            let from = TransitionSubject::LocatedNativeBytes {
                value: b"s".to_vec(),
                argument_index: 0,
            };
            let to = TransitionSubject::LocatedNativeBytes {
                value: b"::N::held".to_vec(),
                argument_index: 1,
            };
            let qualifier = TransitionSubject::LocatedNativeBytes {
                value: b"::N".to_vec(),
                argument_index: 1,
            };
            let mut transitions = StateTransitions::default();
            transitions.push(StateTransition::Namespace(NamespaceTransition::Ensure {
                namespace: NamespaceTransitionTarget::Named(qualifier.clone()),
            }));
            transitions.push(StateTransition::CommandBinding(
                CommandBindingTransition::Move { from, to },
            ));
            assert!(original_transition_subject_matches(
                &qualifier,
                input,
                &transitions
            ));
            assert_eq!(input.bytes(), b"::N::held");
            let mut unrelated = StateTransitions::default();
            unrelated.push(StateTransition::Namespace(NamespaceTransition::Ensure {
                namespace: NamespaceTransitionTarget::Named(qualifier.clone()),
            }));
            assert!(!original_transition_subject_matches(
                &qualifier, input, &unrelated
            ));
            let foreign = TransitionSubject::LocatedNativeBytes {
                value: b"::Other".to_vec(),
                argument_index: 1,
            };
            assert!(!original_transition_subject_matches(
                &foreign,
                input,
                &transitions
            ));
        }
    }

    fn last_words(
        source: &str,
        dialect: &str,
    ) -> Option<crate::registry_invocation::source_structure::OriginalRegistryWords> {
        let analysis = Analyser::new().analyse(source, dialect);
        let config = analysis.body_lexer_config.unwrap();
        let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        source_registry_words(source, &analysis, commands.last().unwrap())
    }

    #[test]
    fn authored_source_catalogue_barriers_keep_known_shadow_and_unloaded_advice_distinct() {
        // Implementation contract: naming.source.authored-command-transition-advice
        // docs/design/analysis/name-resolution-proofs/authored-command-transition-advice.md
        for source in [
            "package require Tk 8.6; proc entry args {}; entry .e -placeholder text",
            "package require Tk 8.6; proc entry args {}; rename entry {}; entry .e -placeholder text",
            "package require Tk 8.6; proc entry args {}; rename entry moved; entry .e -placeholder text",
        ] {
            assert!(last_words(source, "tcl8.6").is_none(), "{source}");
        }
        for source in [
            "package require Tk 8.6; entry .e -placeholder text",
            "package require Tk 8.6; mystery; entry .e -placeholder text",
        ] {
            assert!(last_words(source, "tcl8.6").is_some(), "{source}");
        }
    }

    #[test]
    fn authored_source_tape_refusals_do_not_reissue_stock_declaration_advice() {
        // Implementation contract: naming.source.authored-command-transition-advice
        // docs/design/analysis/name-resolution-proofs/authored-command-transition-advice.md
        assert_eq!(
            last_words("if {1} {puts original}", "tcl")
                .unwrap()
                .command(),
            "if"
        );
        for source in [
            "proc if {args} {}; if {1} {puts shadowed}",
            "rename if {}; if {1} {puts deleted}",
            "rename if copied; if {1} {puts moved}",
        ] {
            assert!(last_words(source, "tcl").is_none(), "{source}");
        }
        assert_eq!(
            last_words("rename if copied; copied {1} {puts original}", "tcl")
                .unwrap()
                .command(),
            "if"
        );
        assert!(logical_last_body_words("proc p {} {if {1} {puts original}}").is_some());
        assert!(
            logical_last_body_words("proc p {} {proc if {args} {}; if {1} {puts shadowed}}")
                .is_none()
        );
        assert!(
            logical_last_body_words("proc p {} {rename if {}; if {1} {puts deleted}}").is_none()
        );
    }

    fn logical_last_body_words(
        source: &str,
    ) -> Option<crate::registry_invocation::source_structure::OriginalRegistryWords> {
        let analysis = Analyser::new().analyse(source, "tcl");
        let config = analysis.body_lexer_config?;
        let context = analysis.resolved_input.as_ref()?.context_registry();
        let roots = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        let root = source_registry_words(source, &analysis, roots.first()?)?;
        let bodies = root.source_script_bodies(&context);
        let span = bodies.first()?.content_span();
        let children = crate::segmenter::segment_commands_with_offset_and_config(
            source.get(span.as_range())?,
            span.start(),
            config,
        );
        source_registry_words(source, &analysis, children.last()?)
    }

    #[test]
    fn authored_source_transitions_keep_order_prefixes_and_conditional_authority() {
        // Implementation contract: naming.source.authored-command-transition-advice
        // docs/design/analysis/name-resolution-proofs/authored-command-transition-advice.md
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let source =
                "opaque\ninterp alias {} pick {} switch -exact\npick subject {default {puts ok}}";
            let mut analysis = Analyser::new().analyse(source, dialect);
            analysis.command_invocations.clear();
            let config = analysis.body_lexer_config.unwrap();
            let commands =
                crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
            assert!(
                source_registry_words(source, &analysis, &commands[0]).is_none(),
                "{dialect}"
            );
            let words = source_registry_words(source, &analysis, &commands[2]).expect(dialect);
            assert_eq!(words.command(), "switch");
            assert_eq!(
                words.origins(),
                [
                    InvocationWordOrigin::Written(0),
                    InvocationWordOrigin::BindingPrefix(0),
                    InvocationWordOrigin::Written(1),
                    InvocationWordOrigin::Written(2)
                ]
            );
            assert!(words.operands()[0].is_none());
            assert_eq!(
                words.operands()[1].as_ref().unwrap().span(),
                commands[2].argv[1].span
            );
            assert!(!words.operands_preserve_source_lookup());
            let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
                panic!("unavailable lookup borrowed another source purpose: {dialect}");
            };
            assert_eq!(advice.lineage().len(), 1);
            assert_eq!(advice.lineage()[0].original_words().len(), 7);
            assert_eq!(
                advice.uncertain_operations()[0][0].span(),
                commands[0].argv[0].span
            );
            assert!(
                advice
                    .obligations()
                    .contains(&super::SourceCommandTransitionObligation::UnknownEarlierMutation)
            );
            assert!(analysis.original_completed_command_world().is_none());
            let context = analysis.resolved_input.as_ref().unwrap().context_registry();
            assert!(
                words
                    .with_source_schema(&context, |schema| schema
                        .authored_source_case_invocation()
                        .is_some())
                    .unwrap()
            );
            assert_eq!(words.source_script_bodies(&context).len(), 1);
            assert!(
                source_registry_words(&format!("{source}\n# stale"), &analysis, &commands[2])
                    .is_none()
            );
            let foreign =
                tcl_registry::model::ingress::static_context_for(if dialect == "tcl8.4" {
                    "tcl8.5"
                } else {
                    "tcl8.4"
                });
            assert!(words.with_source_schema(foreign, |_| ()).is_none());
            let mut changed = analysis.clone();
            let mut same_grammar = config;
            same_grammar.leading_bom = match config.leading_bom {
                tcl_lexer::LeadingBom::Content => tcl_lexer::LeadingBom::Skip,
                tcl_lexer::LeadingBom::Skip => tcl_lexer::LeadingBom::Content,
            };
            changed.body_lexer_config = Some(same_grammar);
            assert!(source_registry_words(source, &changed, &commands[2]).is_none());
            if dialect == "tcl8.6" {
                let (_owner, native_entry) =
                    crate::environment_ingress::captured_native_entry_with_owner(
                        tcl_dialect::DialectProfile::find(dialect).unwrap(),
                    );
                let entry = std::sync::Arc::new(crate::command_binding::SourceAnalysisEntry {
                    native_entry: Some(std::sync::Arc::new(native_entry)),
                    ..Default::default()
                });
                let supplied = Analyser::new()
                    .with_source_analysis_entry(entry)
                    .analyse(source, dialect);
                let supplied = source_registry_words(source, &supplied, &commands[2]);
                assert!(
                    supplied.is_none_or(|words| !matches!(
                        words.source(),
                        OriginalRegistrySource::SourceTransitions(_)
                    )),
                    "a supplied current entry cannot borrow fresh authored transition advice"
                );
            }
        }
    }

    #[test]
    fn authored_immediate_bodies_keep_mutation_and_callback_obligations_separate() {
        // Implementation contract: naming.source.original-immediate-body-mutation-advice
        // docs/design/analysis/name-resolution-proofs/original-immediate-body-mutation-advice.md
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = version.dialect_name();
            let prefix =
                "opaque; interp alias {} first {} switch; interp alias {} second {} switch; ";
            let source = format!(
                "{prefix}first subject {{default {{puts child}}}}; second subject {{default {{puts later}}}}"
            );
            let analysis = Analyser::new().analyse(&source, dialect);
            let config = analysis.body_lexer_config.unwrap();
            let commands =
                crate::segmenter::segment_commands_with_offset_and_config(&source, 0, config);
            let words =
                source_registry_words(&source, &analysis, commands.last().unwrap()).expect(dialect);
            let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
                panic!("source purpose changed");
            };
            assert_eq!(advice.lineage().len(), 1);
            assert!(
                advice
                    .obligations()
                    .contains(&super::SourceCommandTransitionObligation::UnknownEarlierMutation)
            );
            let child = u32::try_from(source.find("puts child").unwrap()).unwrap();
            assert!(advice.uncertain_operations().iter().any(|words| {
                words
                    .first()
                    .is_some_and(|word| word.span().start() == child)
            }));
            assert!(!words.operands_preserve_source_lookup());
            assert!(analysis.original_completed_command_world().is_none());
            let context = analysis.resolved_input.as_ref().unwrap().context_registry();
            assert_eq!(words.source_script_bodies(&context).len(), 1);

            // Complete nested source transitions withdraw the prior alias,
            // including a command substitution that runs before its operand.
            for body in [
                "rename second {}",
                "rename second moved",
                "interp alias {} second {} puts",
                "if {1} {rename second {}}",
                "puts [rename second {}]",
                "if {[rename second {}] eq {}} {puts expression}",
                "if {0 && [rename second {}]} {puts lazy}",
            ] {
                let source = format!(
                    "{prefix}first subject {{default {{{body}}}}}; second subject {{default {{puts stale}}}}"
                );
                assert!(last_words(&source, dialect).is_none(), "{dialect}: {body}");
            }
            let registration = if dialect == "tcl8.4" {
                "trace variable watched r hook"
            } else {
                "trace add variable watched read hook"
            };
            let source = format!(
                "{prefix}first subject {{default {{{registration}; set watched}}}}; second subject {{default {{puts observed}}}}"
            );
            let words = last_words(&source, dialect);
            if let Some(words) = words {
                let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
                    panic!("observer borrowed current selection");
                };
                assert!(
                    advice.obligations().contains(
                        &super::SourceCommandTransitionObligation::UnknownEarlierMutation
                    )
                );
                let observer = u32::try_from(source.find(registration).unwrap()).unwrap();
                assert!(
                    advice.uncertain_operations().iter().any(|words| {
                        words
                            .first()
                            .is_some_and(|word| word.span().start() == observer)
                    }),
                    "{dialect}: registration={observer}, uncertain={:?}",
                    advice
                        .uncertain_operations()
                        .iter()
                        .map(|words| words.first().map(tcl_lexer::NativeWord::span))
                        .collect::<Vec<_>>()
                );
                assert!(!words.operands_preserve_source_lookup());
            }
        }
    }

    #[test]
    fn authored_dynamic_operands_keep_source_advice_before_the_future_barrier() {
        // Implementation contract: naming.source.original-immediate-body-mutation-advice
        // docs/design/analysis/name-resolution-proofs/original-immediate-body-mutation-advice.md
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = version.dialect_name();
            let source = "opaque; interp alias {} pick {} switch; pick $value {default {puts body}}; pick subject {default {puts after}}";
            let analysis = Analyser::new().analyse(source, dialect);
            let config = analysis.body_lexer_config.unwrap();
            let commands =
                crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
            let words = source_registry_words(source, &analysis, &commands[2]).expect(dialect);
            let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
                panic!("dynamic argv changed source purpose");
            };
            assert!(
                advice
                    .obligations()
                    .contains(&super::SourceCommandTransitionObligation::UnknownEarlierMutation)
            );
            assert!(
                advice
                    .uncertain_operations()
                    .iter()
                    .any(|words| words.as_ref() == advice.original_words())
            );
            assert_eq!(
                words.arguments()[0],
                crate::registry_invocation::EffectiveInvocationWord::Dynamic
            );
            assert!(!words.operands_preserve_source_lookup());
            assert!(source_registry_words(source, &analysis, &commands[3]).is_none());
            let context = analysis.resolved_input.as_ref().unwrap().context_registry();
            assert_eq!(words.source_script_bodies(&context).len(), 1);
            let source = "opaque; interp alias {} pick {} switch; pick [rename pick {}] {default {puts deleted}}";
            assert!(last_words(source, dialect).is_none());
        }
    }

    #[test]
    fn authored_source_transition_tape_keeps_deletion_rebinding_and_namespace_geometry() {
        // Implementation contract: naming.source.authored-command-transition-advice
        // docs/design/analysis/name-resolution-proofs/authored-command-transition-advice.md
        for source in [
            "opaque; interp alias {} pick {} switch; pick subject {default {puts yes}}",
            "opaque; interp alias {} pick {} puts; interp alias {} pick {}; interp alias {} pick {} switch; pick subject {default {puts yes}}",
            "opaque; rename switch pick; pick subject {default {puts yes}}",
            "opaque; namespace eval ::A {}; interp alias {} ::A::pick {} ::switch; ::A::pick subject {default {puts yes}}",
            "opaque; interp alias {} a {} switch -exact; interp alias {} b {} a subject; b {default {puts yes}}",
        ] {
            let words = last_words(source, "tcl8.6").unwrap_or_else(|| panic!("{source}"));
            assert_eq!(words.command(), "switch", "{source}");
            assert!(
                matches!(words.source(), OriginalRegistrySource::SourceTransitions(_)),
                "{source}"
            );
            assert!(!words.operands_preserve_source_lookup());
        }
        let chain = last_words(
            "opaque; interp alias {} a {} switch -exact; interp alias {} b {} a subject; b {default {puts yes}}",
            "tcl8.6",
        ).unwrap();
        assert_eq!(
            chain.origins(),
            [
                InvocationWordOrigin::Written(0),
                InvocationWordOrigin::BindingPrefix(0),
                InvocationWordOrigin::BindingPrefix(1),
                InvocationWordOrigin::Written(1)
            ]
        );
        assert_eq!(chain.arguments()[0].literal_bytes(), Some(&b"-exact"[..]));
        assert_eq!(chain.arguments()[1].literal_bytes(), Some(&b"subject"[..]));
        assert!(chain.operands()[0].is_none() && chain.operands()[1].is_none());
        let OriginalRegistrySource::SourceTransitions(advice) = chain.source() else {
            panic!("wrong purpose");
        };
        assert_eq!(advice.lineage().len(), 2);
        let source = "opaque; interp alias {} factory {} interp alias; factory {} pick {} switch; pick subject {default {puts yes}}";
        let operation = last_words(source, "tcl8.6").unwrap();
        assert_eq!(operation.command(), "switch");
        let OriginalRegistrySource::SourceTransitions(advice) = operation.source() else {
            panic!("wrong purpose");
        };
        let transition = &advice.lineage()[0];
        assert_eq!(
            transition.original_words()[0].span().start(),
            u32::try_from(source.find("factory {} pick").unwrap()).unwrap()
        );
        assert_eq!(transition.descriptor(), "interp");
        assert_eq!(transition.selection_lineage().len(), 1);
        assert_eq!(
            transition.selection_lineage()[0].site().offset,
            u32::try_from(source.find("interp alias").unwrap()).unwrap()
        );
        assert_eq!(transition.selection_lineage()[0].original_words().len(), 7);
        for source in [
            "opaque; pick subject {default {puts no}}",
            "opaque; interp alias {} pick {} switch; interp alias {} pick {}; pick subject {default {puts no}}",
            "opaque; rename switch pick; rename pick {}; pick subject {default {puts no}}",
            "opaque; proc switch {args} {}; interp alias {} pick {} switch; pick subject {default {puts no}}",
            "opaque; interp alias {} pick {} switch; proc pick {args} {}; pick subject {default {puts no}}",
            "opaque; interp alias {} pick {} switch; unknown; pick subject {default {puts no}}",
            "opaque; interp alias {} pick {} switch; interp alias {} pick {} $target; pick subject {default {puts no}}",
            "opaque; interp alias {} a {} b; interp alias {} b {} a; a subject {default {puts no}}",
            "opaque; interp alias {} ::missing::pick {} switch; ::missing::pick subject {default {puts no}}",
            "opaque; rename switch puts; puts subject {default {puts no}}",
            "opaque; namespace eval A {interp alias {} pick {} switch}; pick subject {default {puts no}}",
        ] {
            assert!(
                last_words(source, "tcl8.6").is_none_or(|words| !matches!(
                    words.source(),
                    OriginalRegistrySource::SourceTransitions(_)
                )),
                "{source}"
            );
        }
    }

    #[test]
    fn authored_logical_source_advice_keeps_its_explicit_input_and_original_operands() {
        // Implementation contract: naming.source.authored-command-transition-advice
        // docs/design/analysis/name-resolution-proofs/authored-command-transition-advice.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let context = tcl_registry::model::ingress::context_for_profile(profile);
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input =
            crate::analyser::ResolvedAnalysisInput::new(profile, profile, context.clone(), config);
        for source in [
            "pick subject {default {puts before}}; interp alias {} pick {} switch -exact; pick subject {default {puts after}}",
            "pick subject {default {puts before}}; rename switch pick; pick subject {default {puts after}}",
            "::N::pick subject {default {puts before}}; namespace eval N {}; interp alias {} ::N::pick {} ::switch; ::N::pick subject {default {puts after}}",
        ] {
            let syntax = crate::registry_invocation::source_structure::OriginalSourceRegistryContext::capture(source, input.clone());
            let commands =
                crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
            assert!(syntax.words(&commands[0]).is_none());
            let foreign_generation =
                context.with_command_store(context.commands().snapshot().shared_registry());
            let mut tokens = crate::ir::CommandTokens::from_segmented(
                &tcl_lexer::SourceMap::new(source),
                config,
                commands.last().unwrap(),
            );
            syntax
                .retained_source_realm()
                .stamp_original_tokens(&mut tokens);
            assert!(
                syntax
                    .retained_source_realm()
                    .original_source_transition_advice(&foreign_generation, &tokens)
                    .is_none()
            );
            // A rejected equivalent foreign generation cannot poison the
            // actual input's shared source-advice cache.
            let words = syntax
                .words(commands.last().unwrap())
                .expect("explicit Logical source transition advice");
            let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
                panic!("source purpose changed");
            };
            assert!(advice.original_head().native_input().is_none());
            assert!(advice.original_head().logical_input().is_some());
            assert_eq!(advice.logical_source_input(), Some(syntax.editing_input()));
            assert!(words.head_source().unwrap().input().is_none());
            assert!(!words.operands_preserve_source_lookup());
            assert!(!advice.matches_context(&foreign_generation));
            assert!(
                words
                    .with_source_schema(&foreign_generation, |_| ())
                    .is_none()
            );
            assert!(
                advice
                    .lineage()
                    .iter()
                    .flat_map(|step| step.inputs())
                    .all(|(_, input)| input.native_input().is_none())
            );
            assert!(
                words
                    .source_script_bodies(&context)
                    .iter()
                    .any(|body| source.get(body.content_span().as_range()) == Some("puts after"))
            );
            assert!(
                words
                    .with_source_schema(
                        &tcl_registry::model::ingress::context_for_profile(
                            tcl_dialect::DialectProfile::find("tcl9.0").unwrap()
                        ),
                        |_| ()
                    )
                    .is_none()
            );
        }
        for source in [
            "opaque; interp alias {} pick {} switch; interp alias {} pick {}; pick subject {default {puts deleted}}",
            "opaque; interp alias {} pick {} switch; proc pick {args} {}; pick subject {default {puts shadowed}}",
            "opaque; interp alias {} pick {} switch; unknown; pick subject {default {puts changed}}",
            "opaque; interp alias {} pick {} $target; pick subject {default {puts dynamic}}",
        ] {
            let syntax = crate::registry_invocation::source_structure::OriginalSourceRegistryContext::capture(source, input.clone());
            let commands =
                crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
            assert!(syntax.words(commands.last().unwrap()).is_none(), "{source}");
        }
    }

    #[test]
    fn authored_jim_root_alias_advice_keeps_the_jim_name_recipe() {
        // Implementation contract: naming.source.authored-command-transition-advice
        // docs/design/analysis/name-resolution-proofs/authored-command-transition-advice.md
        let source = "opaque; alias pick puts; pick hello";
        let words = last_words(source, "jim").unwrap();
        assert_eq!(words.command(), "puts");
        let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
            panic!("wrong purpose");
        };
        assert_eq!(
            advice
                .original_head()
                .native_input()
                .unwrap()
                .policy()
                .recipe(),
            tcl_syntax::naming::NativeNameProtocol::Jim084
        );
        assert_eq!(advice.lineage()[0].descriptor(), "alias");
        assert_eq!(advice.lineage()[0].original_words().len(), 3);
        assert_eq!(
            advice.lineage()[0].inputs().len(),
            2,
            "implicit interpreter constants have no original source operands"
        );
        assert!(
            advice.lineage()[0]
                .inputs()
                .iter()
                .all(|(subject, _)| subject.argument_index().is_some())
        );
    }
}
