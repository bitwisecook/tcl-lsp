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

//! Authored command transitions retain source advice independently of execution.

mod body_effects;
mod catalogue_seed;
pub(super) use catalogue_seed::AdviceCatalogueSeedMemo;
mod interpreter_handle;
pub use interpreter_handle::OriginalSourceInterpreterHandleBody;
mod interpreter_path;
pub use interpreter_path::OriginalSourceInterpreterPathBinding;
mod c_namespace;
mod logical_lambda_formatting;
pub use logical_lambda_formatting::OriginalConditionalLambdaFormattingBody;
mod logical_body;
mod source_parameter_template;
pub(crate) use source_parameter_template::OriginalLogicalParameterSourcePrefix;
mod logical_declaration;
mod logical_math;
mod logical_name_reference;
pub use logical_name_reference::OriginalLogicalNameReference;
mod logical_procedure_invocation;
pub use logical_procedure_invocation::OriginalLogicalProcedureCall;
mod logical_navigation;
pub use logical_math::OriginalLogicalMathFunctionOccurrence;
mod logical_word_geometry;
pub use logical_navigation::{
    LogicalExternalClassMemberSelector, OriginalLogicalAbsoluteCommandCandidate,
    OriginalLogicalCallbackCandidates, OriginalLogicalCommandCandidates,
};
mod native_baseline;
mod produced_prefix;
mod registered_instance;
mod source_callback;
mod source_point;
pub(super) use source_point::OriginalSourcePointAdviceGraph;
mod source_class;
mod source_jim_member;
pub use source_jim_member::OriginalAuthoredJimProcedureMember;
mod source_jim_class;
pub use source_jim_class::{OriginalAuthoredJimClass, OriginalAuthoredJimClassCall};
mod source_constructor_effects;
pub use source_constructor_effects::{
    OriginalSourceConstructorBodyAttempt, OriginalSourceConstructorBodyCoverage,
    OriginalSourceConstructorBodyDisposition,
};
mod source_class_publications;
mod source_class_reference;
mod source_configured_class;
mod source_publication_proposal;
pub use source_publication_proposal::OriginalSourcePublicationProposal;
mod source_relation_provider;
pub(crate) use source_relation_provider::LogicalSourceClassRelationProvider;
pub use source_relation_provider::{
    LogicalSourceClassRelationCandidates, LogicalSourceClassRelationTarget,
    OriginalSourceClassRelationProvider,
};
mod source_namespace;
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
mod native_autoload_source;
pub use native_autoload_source::{
    OriginalNativeDeferredAutoloadAdvice, OriginalNativeDeferredAutoloadObligation,
    source_native_deferred_autoload_advice_at,
};
mod native_declared_body;
mod source_instance;
mod source_procedure;
pub(crate) use native_declared_body::OriginalNativeSourceBodyChildren;
pub(super) use native_declared_body::{
    OriginalNativeSourceCommand, OriginalNativeSourceDeclaration,
};
mod source_procedure_publications;
pub use produced_prefix::{
    OriginalSourceAuthoredCommandPrefix, OriginalSourceProducedCommandPrefix,
};
pub(crate) use produced_prefix::{
    original_single_command_substitution_words, original_single_command_substitution_words_in_plan,
};
pub use registered_instance::{
    OriginalSourceRegisteredHandleBinding, OriginalSourceRegisteredInstance,
    OriginalSourceRegisteredInstanceWords,
};
pub use source_class::{
    OriginalSourceClassDeclaration, OriginalSourceClassProcCall, OriginalSourceConstructorCall,
    OriginalSourceConstructorShape,
};
pub use source_instance::{
    OriginalSourceClassHandleBinding, OriginalSourceClassInstance, OriginalSourceClassInstanceWords,
};
pub use source_procedure_publications::{
    OriginalSourceProcedureCandidate, OriginalSourceProcedurePublication,
    OriginalSourceProcedurePublications,
};

pub use logical_body::OriginalLogicalProcedureBody;
pub use native_declared_body::OriginalNativeProcedureBodyCommand;
pub use source_procedure::OriginalSourceProcedureCall;

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
/// Unicode source presentation. No Native name recipe, value object or slot is issued.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OriginalLogicalSourceNameInput {
    original: NativeWord,
    bytes: Arc<[u8]>,
}
impl OriginalLogicalSourceNameInput {
    pub(super) fn from_original_source_word(
        original: &NativeWord,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Self> {
        if !input.has_logical_source_name_context() || original.config() != input.lexer_config() {
            return None;
        }
        let bytes = tcl_syntax::word_rules::original_static_word_unicode_value(original)?;
        (std::str::from_utf8(&bytes).is_ok() && !bytes.contains(&0)).then(|| Self {
            original: original.clone(),
            bytes: Arc::from(bytes),
        })
    }

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
    pub(crate) fn conditional_display_word(
        word: &NativeWord,
        input: &crate::analyser::ResolvedAnalysisInput,
        policy: Option<NamePolicyProtocol>,
    ) -> Option<Self> {
        (word.config() == input.lexer_config()).then_some(())?;
        let policy = if input.has_logical_source_name_context() {
            policy.is_none().then_some(())?;
            AdviceNamingPolicy::Logical(input.clone())
        } else {
            AdviceNamingPolicy::Native(policy?)
        };
        original_words(std::slice::from_ref(word), &policy)?
            .pop()?
            .input
    }

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
    /// A source schema in this prefix does not accept its frozen argv.
    /// This is unavailable source applicability, not a reached runtime error.
    UnacceptedSourceArity,
    /// Explicit Logical source assistance, without a Native naming/value issuer.
    ExplicitLogicalSourceApplicability,
    /// Actual original declaration entry/frame and before-argv alternatives.
    /// This does not establish that the body or handler is entered.
    ConditionalDeclarationApplicability,
    /// Authentic member-body source catalogue, without a member or frame entry.
    ConditionalMemberBodyCatalogueApplicability,
    /// Authentic original body syntax, independently of current namespace/frame.
    ConditionalOriginalBodyCatalogueApplicability,
    /// A complete Logical source body and its authentic parent schema own
    /// this conditional header. No runtime activation or frame is supplied.
    DeferredLogicalBodyApplicability,
    /// A selected original immediate body supplies conditional source syntax,
    /// without complete operand effects, an entered frame or execution facts.
    ConditionalLogicalBodyApplicability,
    /// C-policy source coordinates describe conditional namespace syntax;
    /// they supply no actual namespace token, path or entered world.
    ConditionalCNamespaceApplicability,
    /// An original C-policy body retains its source parent and applicability,
    /// independently of a runtime activation or successful body entry.
    ConditionalCBodyApplicability,
    /// A projected source path names an unretained provider. Its existence,
    /// successful path installation and runtime command contents are unknown.
    UnretainedCSourcePathProviderApplicability,
    /// A source lifecycle uses its authored caller-scope descriptor only
    /// conditionally; provider installation, phase entry and effects are unknown.
    ConditionalCapturedLifecycleSourceApplicability,
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

/// Genuine script/member/expression issuers for conditional readonly schemas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OriginalSourceCatalogueBody {
    Script(Arc<crate::registry_invocation::OriginalSourceScriptBody>),
    Member(Arc<crate::registry_invocation::OriginalSourceDefinitionMemberScriptBody>),
    Expression(Arc<body_effects::OriginalSourceExpressionCommand>),
}
impl OriginalSourceCatalogueBody {
    pub(crate) fn content_span(&self) -> tcl_lexer::Span {
        match self {
            Self::Script(body) => body.content_span(),
            Self::Member(body) => body.content_span(),
            Self::Expression(body) => body.content_span(),
        }
    }
    pub(crate) fn matches_source(&self, source: &SourceImage, config: LexerConfig) -> bool {
        match self {
            Self::Script(body) => body.matches_source(source, config),
            Self::Member(body) => body.matches_source(source, config),
            Self::Expression(body) => body.matches_source(source, config),
        }
    }
    pub(crate) fn matches_context(&self, context: &ContextRegistry) -> bool {
        match self {
            Self::Script(body) => body.matches_context(context),
            Self::Member(body) => body.matches_context(context),
            Self::Expression(body) => body.matches_context(context),
        }
    }
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
    logical_scope: Option<Arc<crate::command_binding::OriginalDeclaredLogicalBodyContext>>,
    c_source_body: Option<Arc<crate::registry_invocation::OriginalSourceScriptBody>>,
    c_source_namespace: Option<ByteNamespacePath>,
    c_source_path: Option<Vec<ByteNamespacePath>>,
    procedure_body: Option<Arc<crate::registry_invocation::OriginalSourceScriptBody>>,
    member_catalogue_body:
        Option<Arc<crate::registry_invocation::OriginalSourceDefinitionMemberScriptBody>>,
    catalogue_body: Option<OriginalSourceCatalogueBody>,
}
#[cfg(debug_assertions)]
fn trace_jim_member_earlier_operation(
    contributor: &str,
    contributor_site: u32,
    index: usize,
    total: usize,
    operation: &[NativeWord],
) {
    let stage = "jim.own_member.earlier_operation";
    if !crate::registry_invocation::source_structure::jim_source_trace_row(stage) {
        return;
    }
    let Some(head) = operation.first() else {
        eprintln!(
            "JIM_SOURCE_OWNER stage={stage} contributor={contributor} contributor_site={contributor_site} index={index} total={total} words=0 state=NO_HEAD"
        );
        return;
    };
    let source = head.image().bytes();
    let written = head.written_bytes();
    eprintln!(
        "JIM_SOURCE_OWNER stage={stage} contributor={contributor} contributor_site={contributor_site} index={index} total={total} words={} source_data={:p} source_len={} source_complete={} source_prefix={:?} start={} end={} head_len={} head_complete={} head_prefix={:?}",
        operation.len(),
        source.as_ptr(),
        source.len(),
        source.len() <= 64,
        &source[..source.len().min(64)],
        head.span().start(),
        head.span().end(),
        written.len(),
        written.len() <= 32,
        &written[..written.len().min(32)],
    );
}

impl OriginalSourceCommandTransitionAdvice {
    /// Failure-only held fields; no lookup, analysis or applicability grant.
    #[cfg(debug_assertions)]
    pub(super) fn trace_jim_member_earlier(&self, contributor: &str) {
        let stage = "jim.own_member.earlier_summary";
        if !crate::registry_invocation::source_structure::jim_source_trace_row(stage) {
            return;
        }
        let debt = self.obligations.iter().any(|obligation| {
            matches!(
                obligation,
                SourceCommandTransitionObligation::UnknownEarlierMutation
            )
        });
        let count = self.uncertain_operations.len();
        eprintln!(
            "JIM_SOURCE_OWNER stage={stage} contributor={contributor} contributor_source={:p} contributor_site={} earlier_debt_present={debt} earlier_count={count} shown={} complete={}",
            Arc::as_ptr(&self.site.source),
            self.site.offset,
            count.min(4),
            count <= 4,
        );
        for (index, operation) in self.uncertain_operations.iter().take(4).enumerate() {
            trace_jim_member_earlier_operation(
                contributor,
                self.site.offset,
                index,
                count,
                operation,
            );
        }
    }

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
    /// Whether two advice values retain the same issued original vector.
    #[expect(
        dead_code,
        reason = "only used by the unwired `SourceReadCatalogue::owns_original_advice`"
    )]
    pub(super) fn shares_original_record(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.original, &other.original)
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
            && self.logical_scope.as_ref().is_none_or(|scope| {
                self.logical_input
                    .as_ref()
                    .is_some_and(|input| scope.matches_input(input))
                    && scope.owns_words(&self.original)
                    && scope.body().matches_source(image, config)
            })
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
            && self.c_source_body.as_ref().is_none_or(|body| {
                body.matches_source(image, config)
                    && self.original.iter().all(|word| {
                        word.span().start() >= body.content_span().start()
                            && word.span().end() <= body.content_span().end()
                    })
            })
            && self.catalogue_body.as_ref().is_none_or(|body| {
                body.matches_source(image, config)
                    && self.original.iter().all(|word| {
                        word.span().start() >= body.content_span().start()
                            && word.span().end() <= body.content_span().end()
                    })
            })
            && self.member_catalogue_body.as_ref().is_none_or(|body| {
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
                .c_source_body
                .as_ref()
                .is_none_or(|body| body.matches_context(context))
            && self
                .catalogue_body
                .as_ref()
                .is_none_or(|body| body.matches_context(context))
            && self
                .member_catalogue_body
                .as_ref()
                .is_none_or(|body| body.matches_context(context))
            && self
                .logical_input
                .as_ref()
                .is_none_or(|input| std::ptr::eq(Arc::as_ptr(&input.context_registry()), context))
    }
    /// A retained source descriptor excludes a declared-body dependency only
    /// within its named Logical model, without fallback, alias or mutation debt.
    pub(crate) fn has_named_only_logical_summary_descriptor(&self) -> bool {
        self.logical_input.is_some()
            && self.logical_scope.is_some()
            && self.declaration.is_none()
            && self.lineage.is_empty()
            && self.uncertain_operations.is_empty()
            && !self.obligations.iter().any(|obligation| {
                matches!(
                    obligation,
                    SourceCommandTransitionObligation::UnknownEarlierMutation
                        | SourceCommandTransitionObligation::UnacceptedSourceArity
                        | SourceCommandTransitionObligation::ConditionalDeclarationApplicability
                )
            })
    }

    pub(crate) fn logical_source_scope(
        &self,
    ) -> Option<&Arc<crate::command_binding::OriginalDeclaredLogicalBodyContext>> {
        self.logical_scope.as_ref()
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
    /// Exact retained callable member-body lineage for readonly source callers.
    /// This does not establish an entered receiver, namespace or lookup frame.
    #[must_use]
    pub fn original_definition_member_source_body(
        &self,
    ) -> Option<&crate::registry_invocation::OriginalSourceDefinitionMemberScriptBody> {
        let catalogue = match self.catalogue_body.as_ref() {
            Some(OriginalSourceCatalogueBody::Member(body)) => Some(body.as_ref()),
            _ => None,
        };
        match (self.member_catalogue_body.as_deref(), catalogue) {
            (Some(direct), Some(catalogue)) => (direct == catalogue).then_some(direct),
            (direct, catalogue) => direct.or(catalogue),
        }
    }

    pub(crate) fn original_source_load_association_body(
        &self,
    ) -> Option<Arc<crate::registry_invocation::OriginalSourceScriptBody>> {
        let catalogue = match self.catalogue_body.as_ref() {
            Some(OriginalSourceCatalogueBody::Script(body)) => Some(body),
            _ => None,
        };
        let mut bodies = [
            self.procedure_body.as_ref(),
            self.c_source_body.as_ref(),
            self.logical_body.as_ref(),
            catalogue,
        ]
        .into_iter()
        .flatten();
        let first = bodies.next()?;
        bodies.all(|body| body == first).then(|| Arc::clone(first))
    }

    pub(crate) fn original_catalogue_source_body(&self) -> Option<&OriginalSourceCatalogueBody> {
        self.catalogue_body.as_ref()
    }

    /// Conditional C source coordinates, without a native namespace token.
    #[must_use]
    pub fn conditional_c_source_namespace(&self) -> Option<&ByteNamespacePath> {
        self.c_source_namespace.as_ref()
    }
    /// Ordered source path geometry. Unretained providers remain obligations;
    /// an unavailable path stays None rather than becoming an empty path.
    #[must_use]
    pub fn conditional_c_source_path(&self) -> Option<&[ByteNamespacePath]> {
        self.c_source_path.as_deref()
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
    AuthoredJimClass(Arc<OriginalAuthoredJimClass>),
    SourceProcedure(Arc<source_procedure::OriginalSourceProcedureBody>),
    LogicalSourceProcedure {
        definition: Arc<OriginalSourceCommandTransition>,
        lineage: Vec<Arc<OriginalSourceCommandTransition>>,
    },
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
#[derive(Debug, Clone, PartialEq, Eq)]
struct OriginalLogicalSummaryScope {
    original: Arc<[NativeWord]>,
    scope: Arc<super::OriginalDeclaredLogicalBodyContext>,
}

/// A selected source scope was attempted but its complete header/body walk
/// could not be retained. Positive cards from other scopes do not close it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum LogicalProcedureCatalogueRefusal {
    SelectedScopeUnavailable(u32),
    RootLexicalTailUnavailable(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OriginalSourceArityPrefix {
    original: Arc<[NativeWord]>,
    debts: Vec<Arc<OriginalSourceCommandTransitionAdvice>>,
}

/// A primary declaration-operand value view from one final retained tape row.
/// No advice or registry-word unwrap is exposed to command-selection callers.
pub(crate) struct OriginalSourceProcedureDeclarationValues {
    advice: Arc<OriginalSourceCommandTransitionAdvice>,
    shape: tcl_registry::AuthoredSourceProcedureArguments,
    input: crate::analyser::ResolvedAnalysisInput,
}
impl OriginalSourceProcedureDeclarationValues {
    pub(crate) fn head_correspondence(
        self,
        source: &str,
        config: LexerConfig,
        input: &crate::analyser::ResolvedAnalysisInput,
        selected: &crate::registry_invocation::source_structure::OriginalRegistryWords,
    ) -> Option<crate::registry_invocation::source_structure::OriginalSourceHeadCorrespondence>
    {
        use crate::registry_invocation::{
            InvocationWordOrigin, source_cut_refuses, source_cut_stage,
        };
        if source_cut_refuses(
            "declaration-values.input",
            &self.input != input || config != input.lexer_config(),
        ) {
            return None;
        }
        let context = input.borrowed_context_registry();
        let original = source_cut_stage(
            "pair.words",
            crate::registry_invocation::source_structure::source_transition_words_from_advice(
                source,
                config,
                context,
                self.advice.as_ref().clone(),
            ),
        )?;
        let shape = source_cut_stage(
            "declaration-values.schema",
            original.with_source_schema(context, |schema| {
                schema.authored_source_procedure_arguments()
            }),
        )??;
        if source_cut_refuses("declaration-values.shape", shape != self.shape) {
            return None;
        }
        if source_cut_refuses(
            "pair.operands",
            original.origins().len() != selected.origins().len()
                || original
                    .origins()
                    .iter()
                    .enumerate()
                    .any(|(index, origin)| *origin != InvocationWordOrigin::Written(index))
                || (0..selected.arguments().len()).any(|index| {
                    original.original_argument_word(index) != selected.original_argument_word(index)
                }),
        ) {
            return None;
        }
        source_cut_stage(
            "pair.correspondence",
            original.original_source_head_correspondence(selected, context),
        )
    }
}

type ConditionalProcedureDeclarationSelection = (
    Arc<source_procedure::OriginalSourceProcedureBody>,
    tcl_registry::AuthoredSourceProcedureArguments,
);

type ConditionalProcedureDeclarationSelections =
    BTreeMap<u32, Option<ConditionalProcedureDeclarationSelection>>;

/// Derived source selections only; the existing analysis allocation remains
/// the owner. Weak baseline identity avoids retaining a baseline/owner cycle.
#[derive(Debug, Clone)]
pub(super) struct OriginalConditionalProcedureDeclarationMemo {
    baseline: std::sync::Weak<super::BindingBaseline>,
    registry: tcl_registry::RegistrySemanticKey,
    selections: Arc<ConditionalProcedureDeclarationSelections>,
}

fn conditional_procedure_declaration_selections(
    owner: &Arc<super::OriginalSourceAnalysisOwner>,
    seed: &OriginalSourceTransitionSeed,
    context: &ContextRegistry,
) -> Option<Arc<ConditionalProcedureDeclarationSelections>> {
    if !Arc::ptr_eq(&owner.source, &seed.origin)
        || owner.config != seed.config
        || !owner.metadata.is_standalone()
        || owner.metadata != seed.baseline.metadata_context
        || seed.baseline.native_entry.is_some()
        || seed.baseline.logical_source_input.is_some()
        || seed.baseline.vendor_source_input.is_some()
        || seed.baseline.hosted_execution_context.is_some()
        || seed.baseline.unknown_entry
        || !seed.matches_registry(context.commands())
    {
        return None;
    }
    let derived = owner.derived_memo();
    let baseline = Arc::downgrade(&seed.baseline);
    let registry = context.commands().snapshot().semantic_key();
    let matches = |memo: &OriginalConditionalProcedureDeclarationMemo| {
        std::sync::Weak::ptr_eq(&memo.baseline, &baseline) && memo.registry == registry
    };
    if let Some(memo) = derived
        .conditional_procedure_declarations
        .get()
        .filter(|memo| matches(memo))
    {
        return Some(Arc::clone(&memo.selections));
    }
    // The source interpreter runs outside OnceLock publication. It can neither
    // observe a partly built product nor hold a cache lock during recursion.
    let mut tape = seed.prepare_for(context, OriginalSourceCataloguePurpose::CompletePrefix)?;
    let mut selected = std::mem::take(&mut tape.conditional_procedure_declarations);
    for (offset, declaration) in &mut selected {
        if tape.blocks_registry_source(*offset) {
            *declaration = None;
        }
    }
    let selections = Arc::new(selected);
    if derived.conditional_procedure_declarations.get().is_none() {
        let _ = derived.conditional_procedure_declarations.set(
            OriginalConditionalProcedureDeclarationMemo {
                baseline: baseline.clone(),
                registry: registry.clone(),
                selections: Arc::clone(&selections),
            },
        );
    }
    // Another actual baseline must compute its own product. A same-key racing
    // publication may reuse only the genuinely identical owner's selection map.
    Some(
        derived
            .conditional_procedure_declarations
            .get()
            .filter(|memo| matches(memo))
            .map_or(selections, |memo| Arc::clone(&memo.selections)),
    )
}

#[derive(Debug, Default)]
pub(crate) struct OriginalSourceTransitionAdviceTape {
    pub(crate) partial_expression_span: Option<tcl_lexer::Span>,
    arity_prefixes: BTreeMap<u32, Option<OriginalSourceArityPrefix>>,
    decoded_lambda_phases: BTreeMap<u32, Option<Arc<OriginalConditionalLambdaFormattingBody>>>,
    advice: BTreeMap<u32, OriginalSourceCommandTransitionAdvice>,
    declared: BTreeMap<u32, Option<super::DeclaredSourceSelection>>,
    represented: BTreeSet<u32>,
    registry_barriers: BTreeSet<u32>,
    interpreter_bodies: BTreeMap<u32, OriginalSourceInterpreterHandleBody>,
    interpreter_paths: BTreeMap<u32, Vec<OriginalSourceInterpreterPathBinding>>,
    authored_prefixes: BTreeMap<u32, Option<OriginalSourceAuthoredCommandPrefix>>,
    member_catalogue: BTreeMap<u32, Option<OriginalSourceCommandTransitionAdvice>>,
    body_catalogue: BTreeMap<u32, Option<OriginalSourceCommandTransitionAdvice>>,
    conditional_case_syntax: BTreeMap<u32, Option<OriginalSourceCommandTransitionAdvice>>,
    produced_prefixes: BTreeMap<u32, Option<OriginalSourceProducedCommandPrefix>>,
    registered_instances: BTreeMap<u32, Option<OriginalSourceRegisteredInstanceWords>>,
    registered_factories: BTreeMap<u32, Option<Arc<OriginalSourceRegisteredInstance>>>,
    registered_handles: BTreeMap<u32, Option<Arc<OriginalSourceRegisteredHandleBinding>>>,
    class_declarations: BTreeMap<u32, Option<Arc<OriginalSourceClassDeclaration>>>,
    authored_jim_calls: BTreeMap<u32, Option<OriginalAuthoredJimClassCall>>,
    authored_jim_procedure_members: BTreeMap<
        u32,
        Vec<(
            SignatureSourceNameInput,
            Option<source_jim_member::AuthoredJimProcedureMemberRecipe>,
        )>,
    >,
    authored_jim_classes: BTreeMap<u32, Option<Arc<OriginalAuthoredJimClass>>>,
    class_declaration_facets: BTreeMap<u32, Vec<source_class::SourceClassDeclarationFacet>>,
    constructor_calls: BTreeMap<u32, Option<OriginalSourceConstructorCall>>,
    class_proc_calls: BTreeMap<u32, Option<source_class::OriginalSourceClassProcCall>>,
    class_references: BTreeMap<
        u32,
        Vec<(
            SignatureSourceNameInput,
            Option<OriginalSourceClassReference>,
        )>,
    >,
    configured_classes: BTreeMap<u32, Option<OriginalSourceConfiguredClassReference>>,
    procedure_publications: Option<OriginalSourceProcedurePublications>,
    native_procedure_calls: BTreeMap<u32, Option<OriginalSourceProcedureCall>>,
    conditional_procedure_declarations: ConditionalProcedureDeclarationSelections,
    native_procedure_may_commands:
        BTreeMap<u32, Option<native_declared_body::OriginalNativeSourceMayCommand>>,
    native_deferred_autoload_prefixes:
        BTreeMap<u32, Option<native_autoload_source::NativeAutoloadRecordedPrefix>>,
    native_deferred_autoload_horizons:
        BTreeMap<u32, Option<native_autoload_source::NativeAutoloadRecordedHorizon>>,
    logical_procedure_bodies: BTreeMap<u32, Option<Arc<OriginalLogicalProcedureBody>>>,
    logical_procedure_calls: BTreeMap<u32, Option<OriginalLogicalProcedureCall>>,
    logical_ensemble_transition_receipts:
        BTreeMap<u32, Option<Arc<OriginalSourceCommandTransition>>>,
    logical_name_references: BTreeMap<u32, Option<Vec<OriginalLogicalNameReference>>>,
    logical_procedure_catalogue_refusals: BTreeSet<LogicalProcedureCatalogueRefusal>,
    logical_variable_scopes:
        BTreeMap<(u32, u32), Option<Arc<super::OriginalDeclaredLogicalBodyContext>>>,
    logical_summary_scopes: BTreeMap<u32, Option<OriginalLogicalSummaryScope>>,
    logical_variable_words: BTreeMap<u32, Option<logical_navigation::OriginalLogicalVariableWords>>,
    logical_navigation: BTreeMap<u32, Option<OriginalLogicalCommandCandidates>>,
    logical_math: BTreeMap<u32, Option<OriginalLogicalMathFunctionOccurrence>>,
    logical_absolute_navigation: BTreeMap<u32, Option<OriginalLogicalAbsoluteCommandCandidate>>,
    logical_body_words: BTreeMap<u32, Option<logical_word_geometry::OriginalLogicalBodyWords>>,
    logical_callbacks: LogicalSourceCallbacks,
    class_publications: Option<OriginalSourceClassPublications>,
    display_publications:
        Option<crate::registry_invocation::source_structure::OriginalProjectClassCells>,
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

type LogicalSourceCallbacks =
    BTreeMap<u32, Option<(Arc<[NativeWord]>, Vec<OriginalLogicalCallbackCandidates>)>>;

/// A represented refusal is authoritative for this conditional source purpose.
/// Only an unrepresented site can consult a separate declaration-source owner.
#[derive(Debug, Clone, Copy)]
pub(crate) enum OriginalSourceTransitionAdviceLookup<'a> {
    Advice(&'a OriginalSourceCommandTransitionAdvice),
    Refused,
    Unrepresented,
}
impl OriginalSourceTransitionAdviceTape {
    pub(crate) fn native_procedure_calls(
        &self,
    ) -> impl Iterator<Item = &OriginalSourceProcedureCall> {
        self.native_procedure_calls
            .values()
            .filter_map(Option::as_ref)
    }
    pub(crate) fn native_procedure_call_at(
        &self,
        offset: u32,
    ) -> Option<&OriginalSourceProcedureCall> {
        self.native_procedure_calls.get(&offset)?.as_ref()
    }
    pub(crate) fn lookup(&self, offset: u32) -> OriginalSourceTransitionAdviceLookup<'_> {
        if let Some(advice) = self.advice.get(&offset) {
            OriginalSourceTransitionAdviceLookup::Advice(advice)
        } else if self.represented.contains(&offset) {
            OriginalSourceTransitionAdviceLookup::Refused
        } else {
            OriginalSourceTransitionAdviceLookup::Unrepresented
        }
    }

    pub(crate) fn prefix_catalogue_advice(
        &self,
        offset: u32,
    ) -> Option<&OriginalSourceCommandTransitionAdvice> {
        if self.blocks_registry_source(offset) {
            return None;
        }
        if let Some(advice) = self.body_catalogue(offset) {
            return Some(advice);
        }
        match self.lookup(offset) {
            OriginalSourceTransitionAdviceLookup::Advice(advice) => Some(advice),
            OriginalSourceTransitionAdviceLookup::Refused
            | OriginalSourceTransitionAdviceLookup::Unrepresented => None,
        }
    }

    pub(crate) fn logical_procedure_body_cards(&self) -> Vec<Arc<OriginalLogicalProcedureBody>> {
        self.logical_procedure_bodies
            .values()
            .flatten()
            .cloned()
            .collect()
    }

    /// Represented source ensemble operations in original order. Each receipt
    /// keeps its complete argv, Registry transition recipe, input ordinals, and
    /// alias/move lineage. This is a positive receipt list, not a complete
    /// census: empty or partial results prove neither absence nor current state.
    pub(crate) fn logical_ensemble_transition_receipts(
        &self,
    ) -> Option<Vec<Arc<OriginalSourceCommandTransition>>> {
        self.logical_ensemble_transition_receipts
            .values()
            .cloned()
            .collect()
    }

    pub(crate) fn checked_logical_procedure_body_cards(
        &self,
    ) -> Option<Vec<Arc<OriginalLogicalProcedureBody>>> {
        if !self.logical_procedure_catalogue_refusals.is_empty() {
            return None;
        }
        self.logical_procedure_bodies.values().cloned().collect()
    }

    /// Genuine original receiver-body vectors, without receiver lookup or entry.
    /// Conflicting source receipts withdraw this catalogue as a whole.
    pub(super) fn logical_receiver_source_commands(
        &self,
        origin: &Arc<super::SourceOriginId>,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<
        Vec<(
            Arc<crate::registry_invocation::OriginalSourceDefinitionMemberScriptBody>,
            Arc<crate::registry_invocation::OriginalSourceCommandProjection>,
        )>,
    > {
        input.has_logical_source_name_context().then_some(())?;
        let mut commands = Vec::new();
        for (&offset, receipt) in &self.logical_body_words {
            let receipt = receipt.as_ref()?;
            receipt.original_words(origin, input)?;
            let Some(body) = receipt.original_member_body(origin, input) else {
                continue;
            };
            let original = self.logical_original_command_at(origin, input, offset)?;
            commands.push((body, original));
        }
        Some(commands)
    }

    /// Whole original syntax from independent Logical body/header owners.
    /// The selected command/schema may be unavailable; this grants geometry only.
    pub(crate) fn logical_original_command_at(
        &self,
        origin: &Arc<super::SourceOriginId>,
        input: &crate::analyser::ResolvedAnalysisInput,
        offset: u32,
    ) -> Option<Arc<crate::registry_invocation::OriginalSourceCommandProjection>> {
        input.has_logical_source_name_context().then_some(())?;
        let image = origin.source_image();
        let config = input.lexer_config();
        let mut selected: Option<&[NativeWord]> = None;
        if let Some(receipt) = self.logical_body_words.get(&offset) {
            selected = Some(receipt.as_ref()?.original_words(origin, input)?);
        }
        for advice in [
            self.advice.get(&offset),
            self.body_catalogue.get(&offset).and_then(Option::as_ref),
            self.member_catalogue.get(&offset).and_then(Option::as_ref),
        ]
        .into_iter()
        .flatten()
        {
            if !Arc::ptr_eq(&advice.site().source, origin)
                || advice.site().offset != offset
                || advice.logical_source_input() != Some(input)
                || !advice.matches_source(image, config)
                || !advice.matches_context(input.borrowed_context_registry())
            {
                return None;
            }
            if selected.is_some_and(|previous| previous != advice.original_words()) {
                return None;
            }
            selected = Some(advice.original_words());
        }
        let original = selected?;
        if original.first()?.span().start() != offset
            || original
                .iter()
                .any(|word| word.image() != image || word.config() != config)
        {
            return None;
        }
        // Reuse the existing complete-vector projection owner. Original body
        // extents select parser input; reporting strings select nothing.
        let region = tcl_lexer::Span::new(offset, original.last()?.span().end());
        let body = SourceImage::from_bytes(image.bytes().get(region.as_range())?, image.channel());
        let segments = crate::segmenter::segment_commands_image_with_offset_and_config(
            &body,
            offset,
            config.nested(),
        )?;
        let [command] = segments.as_slice() else {
            return None;
        };
        if command.is_partial {
            return None;
        }
        let tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::from_image(image),
            config,
            command,
        );
        let projection = origin.original_lexical_command(tokens.words(), offset, config)?;
        (projection.native_words() == original).then_some(projection)
    }

    pub(crate) fn logical_original_word_observations_at_span(
        &self,
        origin: &Arc<super::SourceOriginId>,
        input: &crate::analyser::ResolvedAnalysisInput,
        span: tcl_lexer::Span,
    ) -> Option<Vec<(NativeWord, usize, super::CommandAllocationSite)>> {
        input.has_logical_source_name_context().then_some(())?;
        if self.logical_body_words.values().any(Option::is_none) {
            return None;
        }
        let offsets = self
            .logical_body_words
            .keys()
            .chain(self.advice.keys())
            .chain(self.body_catalogue.keys())
            .chain(self.member_catalogue.keys())
            .copied()
            .filter(|offset| *offset <= span.start())
            .collect::<BTreeSet<_>>();
        let mut observations = Vec::new();
        for offset in offsets {
            let Some(projection) = self.logical_original_command_at(origin, input, offset) else {
                continue;
            };
            let mut words = projection
                .native_words()
                .iter()
                .enumerate()
                .filter(|(_, word)| {
                    word.span() == span
                        || word.content_span().ok() == Some(span)
                        || word
                            .tokens()
                            .first()
                            .is_some_and(|token| token.span == span)
                });
            let Some((written, word)) = words.next() else {
                continue;
            };
            if words.next().is_some() {
                return None;
            }
            observations.push((
                word.clone(),
                written,
                super::CommandAllocationSite {
                    source: Arc::clone(origin),
                    offset,
                },
            ));
        }
        Some(observations)
    }

    /// Exact Logical catalogue words retain geometry without an entered invocation.
    pub(crate) fn logical_original_word_at_span(
        &self,
        origin: &Arc<super::SourceOriginId>,
        input: &crate::analyser::ResolvedAnalysisInput,
        span: tcl_lexer::Span,
    ) -> Option<(NativeWord, u32)> {
        input.has_logical_source_name_context().then_some(())?;
        let image = origin.source_image();
        let config = input.lexer_config();
        let context = input.borrowed_context_registry();
        let offsets = self
            .advice
            .keys()
            .chain(self.body_catalogue.keys())
            .copied()
            .collect::<BTreeSet<_>>();
        if self.logical_body_words.values().any(Option::is_none) {
            return None;
        }
        let mut selected = self.logical_body_original_word_at_span(origin, input, span);
        // Declared Logical bodies retain whole vectors independently of a
        // selected Registry handler or ordinary invocation.
        for (&offset, receipt) in self.logical_variable_words.range(..=span.start()) {
            let Some(receipt) = receipt else {
                continue;
            };
            let Some(current) = receipt.original_word_at_span(image, input, span) else {
                continue;
            };
            if current.1 != offset
                || selected
                    .as_ref()
                    .is_some_and(|previous| previous != &current)
            {
                return None;
            }
            selected = Some(current);
        }
        for offset in offsets {
            if self.blocks_registry_source(offset) {
                continue;
            }
            let advice = if let Some(advice) = self.body_catalogue(offset) {
                advice
            } else {
                match self.lookup(offset) {
                    OriginalSourceTransitionAdviceLookup::Advice(advice) => advice,
                    OriginalSourceTransitionAdviceLookup::Refused
                    | OriginalSourceTransitionAdviceLookup::Unrepresented => continue,
                }
            };
            if advice.site().offset != offset
                || advice.original_words().first()?.span().start() != offset
                || advice.site().source != *origin
                || advice.logical_source_input() != Some(input)
                || !advice.matches_source(image, config)
                || !advice.matches_context(context)
            {
                continue;
            }
            let mut words = advice.original_words().iter().filter(|word| {
                word.span() == span
                    || word
                        .tokens()
                        .first()
                        .is_some_and(|token| token.span == span)
            });
            let Some(word) = words.next() else {
                continue;
            };
            if words.next().is_some() {
                return None;
            }
            if let OriginalSourceTransitionAdviceLookup::Advice(original) = self.lookup(offset)
                && original.original_words() != advice.original_words()
            {
                return None;
            }
            let current = (word.clone(), offset);
            if selected
                .as_ref()
                .is_some_and(|previous| previous != &current)
            {
                return None;
            }
            selected = Some(current);
        }
        selected
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

    /// Primary operand values only: final conflicts, absence and barriers refuse.
    pub(crate) fn procedure_declaration_values(
        &self,
        owner: &Arc<super::OriginalSourceAnalysisOwner>,
        input: &crate::analyser::ResolvedAnalysisInput,
        offset: u32,
        original: &[NativeWord],
        binding: &super::SourceInvocationBinding,
    ) -> Option<OriginalSourceProcedureDeclarationValues> {
        use crate::registry_invocation::{source_cut_refuses, source_cut_stage};
        if source_cut_refuses(
            "declaration-values.registry-barrier",
            self.blocks_registry_source(offset),
        ) {
            return None;
        }
        let record = source_cut_stage(
            "declaration-values.record",
            self.conditional_procedure_declarations.get(&offset),
        )?;
        let (procedure, shape) = source_cut_stage("declaration-values.unanimity", record.as_ref())?;
        let advice = &procedure.declaration;
        let origin = &owner.source;
        let site = source_cut_stage(
            "declaration-values.binding-site",
            binding.dispatch_site.as_ref(),
        )?;
        if source_cut_refuses(
            "declaration-values.owner-source-context",
            owner.config != input.lexer_config()
                || !Arc::ptr_eq(&site.source, origin)
                || site.offset != offset
                || binding
                    .original_analysis_owner
                    .as_ref()
                    .is_none_or(|actual| !Arc::ptr_eq(actual, owner))
                || !Arc::ptr_eq(&advice.site.source, origin)
                || advice.site.offset != offset
                || !advice.matches_source(origin.source_image(), input.lexer_config())
                || !advice.matches_context(input.borrowed_context_registry())
                || advice.original_words() != original,
        ) {
            return None;
        }
        Some(OriginalSourceProcedureDeclarationValues {
            advice: Arc::clone(advice),
            shape: *shape,
            input: input.clone(),
        })
    }

    pub(crate) fn blocks_registry_source(&self, offset: u32) -> bool {
        self.registry_barriers.contains(&offset)
    }

    pub(crate) fn member_catalogue(
        &self,
        offset: u32,
    ) -> Option<&OriginalSourceCommandTransitionAdvice> {
        self.member_catalogue.get(&offset)?.as_ref()
    }

    pub(crate) fn original_definition_member_source_catalogue(
        &self,
    ) -> impl Iterator<Item = &OriginalSourceCommandTransitionAdvice> {
        self.member_catalogue.values().filter_map(Option::as_ref)
    }

    pub(crate) fn body_catalogue(
        &self,
        offset: u32,
    ) -> Option<&OriginalSourceCommandTransitionAdvice> {
        self.body_catalogue.get(&offset)?.as_ref()
    }

    pub(crate) fn conditional_case_syntax(
        &self,
        offset: u32,
    ) -> Option<&OriginalSourceCommandTransitionAdvice> {
        self.conditional_case_syntax.get(&offset)?.as_ref()
    }

    pub(super) fn retain_logical_summary_scope(
        &mut self,
        original: &[NativeWord],
        scope: &Arc<super::OriginalDeclaredLogicalBodyContext>,
    ) {
        let Some(head) = original.first() else {
            return;
        };
        source_class::merge_receipt(
            &mut self.logical_summary_scopes,
            head.span().start(),
            Some(OriginalLogicalSummaryScope {
                original: Arc::from(original),
                scope: Arc::clone(scope),
            }),
        );
    }

    fn retain_arity_prefix(&mut self, graph: &AdviceGraph, original: &[NativeWord]) {
        let Some(head) = original.first() else {
            return;
        };
        source_class::merge_receipt(
            &mut self.arity_prefixes,
            head.span().start(),
            Some(OriginalSourceArityPrefix {
                original: Arc::from(original),
                debts: graph.arity_debts.clone(),
            }),
        );
    }

    fn extend_prefix_inventory(&mut self, other: &mut Self) {
        merge_receipt_inventory(
            &mut self.conditional_procedure_declarations,
            std::mem::take(&mut other.conditional_procedure_declarations),
        );
        merge_receipt_inventory(
            &mut self.decoded_lambda_phases,
            std::mem::take(&mut other.decoded_lambda_phases),
        );
        merge_receipt_inventory(
            &mut self.native_procedure_calls,
            std::mem::take(&mut other.native_procedure_calls),
        );
        merge_receipt_inventory(
            &mut self.logical_procedure_calls,
            std::mem::take(&mut other.logical_procedure_calls),
        );
        merge_receipt_inventory(
            &mut self.logical_name_references,
            std::mem::take(&mut other.logical_name_references),
        );
        merge_receipt_inventory(
            &mut self.logical_ensemble_transition_receipts,
            std::mem::take(&mut other.logical_ensemble_transition_receipts),
        );
        merge_receipt_inventory(
            &mut self.arity_prefixes,
            std::mem::take(&mut other.arity_prefixes),
        );
        self.logical_procedure_catalogue_refusals
            .extend(std::mem::take(
                &mut other.logical_procedure_catalogue_refusals,
            ));
        merge_receipt_inventory(
            &mut self.native_procedure_may_commands,
            std::mem::take(&mut other.native_procedure_may_commands),
        );
        merge_receipt_inventory(
            &mut self.native_deferred_autoload_prefixes,
            std::mem::take(&mut other.native_deferred_autoload_prefixes),
        );
        merge_receipt_inventory(
            &mut self.native_deferred_autoload_horizons,
            std::mem::take(&mut other.native_deferred_autoload_horizons),
        );
        merge_receipt_inventory(
            &mut self.logical_callbacks,
            std::mem::take(&mut other.logical_callbacks),
        );
        merge_receipt_inventory(
            &mut self.logical_variable_words,
            std::mem::take(&mut other.logical_variable_words),
        );
        merge_receipt_inventory(
            &mut self.logical_math,
            std::mem::take(&mut other.logical_math),
        );
    }

    fn extend_inventory(&mut self, mut other: Self) {
        self.extend_prefix_inventory(&mut other);
        for (offset, candidate) in other.logical_navigation {
            self.merge_logical_navigation(offset, candidate);
        }
        merge_receipt_inventory(
            &mut self.logical_absolute_navigation,
            other.logical_absolute_navigation,
        );
        merge_receipt_inventory(&mut self.logical_body_words, other.logical_body_words);
        merge_receipt_inventory(
            &mut self.logical_summary_scopes,
            other.logical_summary_scopes,
        );
        merge_receipt_inventory(
            &mut self.logical_procedure_bodies,
            other.logical_procedure_bodies,
        );
        merge_receipt_inventory(
            &mut self.logical_variable_scopes,
            other.logical_variable_scopes,
        );
        merge_receipt_inventory(
            &mut self.conditional_case_syntax,
            other.conditional_case_syntax,
        );
        merge_receipt_inventory(&mut self.body_catalogue, other.body_catalogue);
        merge_receipt_inventory(&mut self.member_catalogue, other.member_catalogue);
        self.scanned_procedure_declarations
            .extend(other.scanned_procedure_declarations);
        merge_receipt_inventory(&mut self.class_instance_calls, other.class_instance_calls);
        for (offset, targets) in other.callback_targets {
            for (prefix, target) in targets {
                self.merge_callback_target(offset, prefix, target);
            }
        }
        for (offset, references) in other.authored_jim_procedure_members {
            for (input, recipe) in references {
                self.merge_authored_jim_procedure_member(offset, input, recipe);
            }
        }
        for (offset, references) in other.class_references {
            for (input, receipt) in references {
                self.merge_class_reference(offset, input, receipt);
            }
        }
        merge_receipt_inventory(&mut self.configured_classes, other.configured_classes);
        self.advice.extend(other.advice);
        self.represented.extend(other.represented);
        self.registry_barriers.extend(other.registry_barriers);
        self.interpreter_bodies.extend(other.interpreter_bodies);
        self.interpreter_paths.extend(other.interpreter_paths);
        #[cfg(debug_assertions)]
        self.trace_authored_jim_inventory(&other.authored_jim_classes);
        merge_receipt_inventory(&mut self.authored_jim_classes, other.authored_jim_classes);
        merge_receipt_inventory(&mut self.authored_jim_calls, other.authored_jim_calls);
        self.extend_class_inventory(other.class_declarations, other.constructor_calls);
        self.extend_class_proc_calls(other.class_proc_calls);
        for (_, facets) in other.class_declaration_facets {
            for facet in facets {
                self.retain_class_declaration_facet(facet);
            }
        }
        merge_receipt_inventory(&mut self.registered_factories, other.registered_factories);
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
fn merge_receipt_inventory<K: Ord, T: PartialEq>(
    entries: &mut BTreeMap<K, Option<T>>,
    other: BTreeMap<K, Option<T>>,
) {
    for (offset, receipt) in other {
        source_class::merge_receipt(entries, offset, receipt);
    }
}

#[derive(Clone)]
struct AdviceGraph {
    native_baseline: bool,
    stock_source_effects: Vec<super::jim_stock_class::OriginalJimStockSourceEffects>,
    cells: super::SharedSourceInventory<rustc_hash::FxHashMap<ByteCommandSlot, AdviceCell>>,
    namespaces: BTreeSet<ByteNamespacePath>,
    namespace_exports: BTreeMap<ByteNamespacePath, Vec<NameBytes>>,
    namespace_paths: BTreeMap<ByteNamespacePath, Vec<ByteNamespacePath>>,
    namespace_imports: HashMap<ByteCommandSlot, ByteCommandSlot>,
    unknown_exports: BTreeSet<ByteNamespacePath>,
    unknown_paths: BTreeSet<ByteNamespacePath>,
    policy: AdviceNamingPolicy,
    logical_namespace: Option<ByteNamespacePath>,
    c_source_namespace: Option<ByteNamespacePath>,
    dialect: InvocationDialect,
    uncertain_operations: Vec<Arc<[NativeWord]>>,
    /// Coordinates a possible branch created but did not install. A lookup
    /// may not fall through one of them; they are never cells or headers.
    possible_occupants: BTreeSet<ByteCommandSlot>,
    arity_debts: Vec<Arc<OriginalSourceCommandTransitionAdvice>>,
    procedure_body: Option<Arc<crate::registry_invocation::OriginalSourceScriptBody>>,
    procedure_declarations: Vec<Arc<source_procedure::OriginalSourceProcedureBody>>,
    class_declarations: Vec<Arc<OriginalSourceClassDeclaration>>,
    future_procedure_source: bool,
    procedure_namespace: Option<ByteNamespacePath>,
    procedure_path: Vec<super::CommandAllocationSite>,
    procedure_steps: Arc<std::sync::atomic::AtomicUsize>,
    decoded_phase_steps: Arc<std::sync::atomic::AtomicUsize>,
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
struct SourceAliasCellSelection<'a> {
    cell: &'a AdviceCell,
    slot: ByteCommandSlot,
    prefix: Vec<SourceAdviceWord>,
    lineage: Vec<Arc<OriginalSourceCommandTransition>>,
}

impl AdviceGraph {
    #[cfg(test)]
    fn new(
        context: &ContextRegistry,
        policy: &AdviceNamingPolicy,
        dialect: InvocationDialect,
    ) -> Option<Self> {
        let seed = catalogue_seed::AdviceCatalogueSeed::build(context, policy)?;
        Some(Self::from_catalogue_seed(policy, dialect, &seed))
    }

    fn from_catalogue_seed(
        policy: &AdviceNamingPolicy,
        dialect: InvocationDialect,
        seed: &catalogue_seed::AdviceCatalogueSeed,
    ) -> Self {
        Self {
            native_baseline: false,
            stock_source_effects: Vec::new(),
            cells: seed.cells(),
            namespaces: seed.namespaces(),
            namespace_exports: seed.namespace_exports(),
            namespace_paths: BTreeMap::new(),
            namespace_imports: HashMap::new(),
            unknown_exports: BTreeSet::new(),
            unknown_paths: BTreeSet::new(),
            policy: policy.clone(),
            logical_namespace: matches!(policy, AdviceNamingPolicy::Logical(_))
                .then(ByteNamespacePath::root),
            c_source_namespace: (dialect.family() == Some(tcl_dialect::model::Family::Tcl)
                && matches!(
                    policy.native().map(NamePolicyProtocol::recipe),
                    Some(NativeNameProtocol::C(_))
                ))
            .then(ByteNamespacePath::root),
            dialect,
            uncertain_operations: Vec::new(),
            possible_occupants: BTreeSet::new(),
            arity_debts: Vec::new(),
            handles: Vec::new(),
            class_handles: Vec::new(),
            procedure_body: None,
            procedure_declarations: Vec::new(),
            class_declarations: Vec::new(),
            future_procedure_source: false,
            procedure_namespace: None,
            procedure_path: Vec::new(),
            procedure_steps: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            decoded_phase_steps: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }
    fn native_source_context(&self) -> Option<NativeNameContext<'_>> {
        let policy = self.policy.native()?;
        self.procedure_namespace.as_ref().map_or_else(
            || Some(root_context(policy)),
            |namespace| Some(NativeNameContext::new(namespace)),
        )
    }
    fn lookup_head_key(&self, bytes: &[u8]) -> Option<ByteCommandSlot> {
        if self.c_source_namespace.is_some() && !self.native_baseline {
            return self.c_source_head_key(bytes);
        }
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
        self.lookup_logical_head_key(namespace, bytes)
    }

    fn lookup_logical_head_key(
        &self,
        namespace: &ByteNamespacePath,
        bytes: &[u8],
    ) -> Option<ByteCommandSlot> {
        let local = tcl_syntax::naming::authored_source_command_slot(namespace, bytes)?;
        if !tcl_syntax::naming::is_absolute_bytes(bytes)
            && self.unknown_paths.contains(namespace)
            && !self
                .cells
                .get(&local)
                .is_some_and(|cell| !matches!(cell, AdviceCell::Deleted))
        {
            return None;
        }
        let candidates = self.logical_head_candidates(namespace, bytes)?;
        candidates
            .iter()
            .find(|key| {
                self.cells
                    .get(*key)
                    .is_some_and(|cell| !matches!(cell, AdviceCell::Deleted))
            })
            .cloned()
            .or_else(|| candidates.last().cloned())
    }

    fn logical_head_candidates(
        &self,
        namespace: &ByteNamespacePath,
        bytes: &[u8],
    ) -> Option<Vec<ByteCommandSlot>> {
        tcl_syntax::naming::authored_source_command_candidates(
            namespace,
            self.namespace_paths
                .get(namespace)
                .map_or(&[], Vec::as_slice),
            bytes,
        )
    }

    /// Every coordinate [`Self::lookup_head_key`] may try, in its order.
    fn head_lookup_candidates(&self, bytes: &[u8]) -> Option<Vec<ByteCommandSlot>> {
        if self.c_source_namespace.is_some() && !self.native_baseline {
            return self.c_source_head_candidates(bytes);
        }
        if let Some(namespace) = &self.procedure_namespace {
            let local = self
                .policy
                .native()?
                .recipe()
                .command_lookup_slot(NativeNameContext::new(namespace), bytes)
                .ok()?;
            return Some(vec![local, lookup_key(&self.policy, bytes)?]);
        }
        let Some(namespace) = &self.logical_namespace else {
            return Some(vec![lookup_key(&self.policy, bytes)?]);
        };
        self.logical_head_candidates(namespace, bytes)
    }

    /// Whether `selected` was reached past a coordinate a possible branch
    /// may have occupied, so the actual lookup may stop earlier.
    fn passes_possible_occupant(
        &self,
        candidates: Option<Vec<ByteCommandSlot>>,
        selected: &ByteCommandSlot,
    ) -> bool {
        candidates.is_some_and(|candidates| {
            candidates
                .iter()
                .take_while(|key| *key != selected)
                .any(|key| self.possible_occupants.contains(key))
        })
    }

    /// The coordinate a command lookup selects, refused when it falls through
    /// a possible occupant. Raw [`Self::lookup_head_key`] remains only for
    /// coordinate and header enumeration and for publication.
    fn selected_head_key(&self, bytes: &[u8]) -> Option<ByteCommandSlot> {
        let selected = self.lookup_head_key(bytes)?;
        if self.possible_occupants.is_empty() {
            return Some(selected);
        }
        (!self.passes_possible_occupant(self.head_lookup_candidates(bytes), &selected))
            .then_some(selected)
    }

    /// [`Self::selected_head_key`] for an explicitly selected Logical namespace.
    fn selected_logical_head_key(
        &self,
        namespace: &ByteNamespacePath,
        bytes: &[u8],
    ) -> Option<ByteCommandSlot> {
        let selected = self.lookup_logical_head_key(namespace, bytes)?;
        if self.possible_occupants.is_empty() {
            return Some(selected);
        }
        (!self.passes_possible_occupant(self.logical_head_candidates(namespace, bytes), &selected))
            .then_some(selected)
    }

    fn publication_key(
        &self,
        bytes: &[u8],
        purpose: AdvicePublicationPurpose,
    ) -> Option<ByteCommandSlot> {
        if self.c_source_namespace.is_some()
            && !self.native_baseline
            && self.procedure_namespace.is_none()
        {
            return self.c_source_publication_key(bytes, purpose);
        }
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
            AliasTargetLookup::CallerNamespace => self.selected_head_key(target.bytes()),
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
        let key = self.selected_head_key(head.bytes())?;
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
        self.withdraw_namespace_advice();
        let imports = &self.namespace_imports;
        let keep = |slot: &ByteCommandSlot, cell: &AdviceCell| match cell {
            AdviceCell::Descriptor(_, lineage) => lineage.is_empty() || imports.contains_key(slot),
            AdviceCell::Shadowed
            | AdviceCell::Deleted
            | AdviceCell::Declared(_)
            | AdviceCell::SourceProcedure(_)
            | AdviceCell::LogicalSourceProcedure { .. }
            | AdviceCell::SourceClass(_)
            | AdviceCell::AuthoredJimClass(_)
            | AdviceCell::ClassInstance(_)
            | AdviceCell::RegisteredInstance(_)
            | AdviceCell::Alias { .. } => true,
            AdviceCell::CreatedInterpreter(_) => false,
        };
        self.cells.update_if_needed(
            |cells| cells.iter().any(|(slot, cell)| !keep(slot, cell)),
            |cells| cells.retain(|slot, cell| keep(slot, cell)),
        );
    }
    fn blocks_registry_source(&self, input: &SourceAdviceNameInput) -> bool {
        let Some(start) = self.lookup_head_key(input.bytes()) else {
            return false;
        };
        if !self.possible_occupants.is_empty()
            && self.passes_possible_occupant(self.head_lookup_candidates(input.bytes()), &start)
        {
            return true;
        }
        let mut blocked = false;
        let cycle = tcl_syntax::naming::alias_chain_loops(start, |key| match self.cells.get(key) {
            Some(AdviceCell::Alias { target, lookup, .. }) => {
                let key = self.alias_target_key(target, *lookup);
                // A caller-namespace target refused past a possible occupant
                // keeps the barrier the raw target would have recorded.
                if key.is_none() && matches!(lookup, AliasTargetLookup::CallerNamespace) {
                    blocked = true;
                }
                Ok::<_, ()>(key)
            }
            Some(
                AdviceCell::Shadowed
                | AdviceCell::Deleted
                | AdviceCell::CreatedInterpreter(_)
                | AdviceCell::RegisteredInstance(_)
                | AdviceCell::SourceClass(_)
                | AdviceCell::AuthoredJimClass(_)
                | AdviceCell::ClassInstance(_)
                | AdviceCell::SourceProcedure(_)
                | AdviceCell::LogicalSourceProcedure { .. }
                | AdviceCell::Declared(_),
            ) => {
                blocked = true;
                Ok(None)
            }
            Some(AdviceCell::Descriptor(_, _)) | None => Ok(None),
        });
        blocked || cycle == Ok(true)
    }

    /// An original Logical scope can retain a possible named catalogue after
    /// unclosed effects. Its uncertainties remain attached to the source advice.
    fn readonly_logical_source_candidate(
        &self,
        head: &SourceAdviceNameInput,
        original: &[NativeWord],
        scope: &Arc<super::source_declared_command::OriginalDeclaredLogicalBodyContext>,
    ) -> Option<(
        String,
        Vec<SourceAdviceWord>,
        Vec<Arc<OriginalSourceCommandTransition>>,
    )> {
        let AdviceNamingPolicy::Logical(input) = &self.policy else {
            return None;
        };
        if self.uncertain_operations.is_empty()
            || !scope.matches_input(input)
            || !scope.owns_words(original)
            || self.logical_namespace.as_ref() != Some(scope.namespace_recipe())
            || original.first()?.group().expand
            || head.logical_input().is_none()
            || tcl_syntax::naming::is_qualified(head.bytes())
            || self.blocks_registry_source(head)
        {
            return None;
        }
        let local = tcl_syntax::naming::authored_source_command_slot(
            scope.namespace_recipe(),
            head.bytes(),
        )?;
        let key = tcl_syntax::naming::authored_source_command_slot(
            &ByteNamespacePath::root(),
            head.bytes(),
        )?;
        let AdviceCell::Descriptor(command, lineage) = self.cells.get(&key)? else {
            return None;
        };
        if !lineage.is_empty() {
            return None;
        }
        match self.cells.get(&local) {
            None | Some(AdviceCell::Deleted) => {}
            Some(AdviceCell::Descriptor(local_command, local_lineage))
                if local_command == command && local_lineage.is_empty() => {}
            _ => return None,
        }
        Some((command.clone(), Vec::new(), Vec::new()))
    }

    /// Shared original alias/captured-prefix traversal. Terminal cell kind
    /// remains a separate purpose; this issues no Registry or Native target.
    fn resolve_source_cell(
        &self,
        input: &SourceAdviceNameInput,
    ) -> Option<SourceAliasCellSelection<'_>> {
        let start = self.selected_head_key(input.bytes())?;
        self.resolve_source_cell_at_slot(start)
    }

    fn resolve_source_cell_at_slot(
        &self,
        start: ByteCommandSlot,
    ) -> Option<SourceAliasCellSelection<'_>> {
        if tcl_syntax::naming::alias_chain_loops(start.clone(), |key| match self.cells.get(key) {
            Some(AdviceCell::Alias { target, lookup, .. }) => {
                Ok::<_, ()>(Some(self.alias_target_key(target, *lookup).ok_or(())?))
            }
            _ => Ok(None),
        })
        .ok()?
        {
            return None;
        }
        let mut key = start;
        let mut prefix = Vec::new();
        let mut lineage = Vec::new();
        loop {
            let cell = self.cells.get(&key)?;
            match cell {
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
                _ => {
                    return Some(SourceAliasCellSelection {
                        cell,
                        slot: key,
                        prefix,
                        lineage,
                    });
                }
            }
        }
    }
    fn resolve(
        &self,
        input: &SourceAdviceNameInput,
    ) -> Option<(
        String,
        Vec<SourceAdviceWord>,
        Vec<Arc<OriginalSourceCommandTransition>>,
    )> {
        let mut selected = self.resolve_source_cell(input)?;
        let AdviceCell::Descriptor(command, steps) = selected.cell else {
            return None;
        };
        selected.lineage.extend(steps.iter().cloned());
        Some((command.clone(), selected.prefix, selected.lineage))
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
                        let mut path = if self.c_source_namespace.is_some() && !self.native_baseline
                        {
                            self.c_source_namespace_path(receipt.input(subject)?)?
                        } else {
                            self.logical_namespace.as_ref().map_or_else(
                                || self.policy.namespace_path(receipt.selected_input(subject)?),
                                |namespace| {
                                    tcl_syntax::naming::authored_source_namespace_path(
                                        namespace,
                                        receipt.selected_input(subject)?,
                                    )
                                },
                            )?
                        };
                        loop {
                            self.namespaces.insert(path.clone());
                            let Some(parent) = path.parent() else {
                                break;
                            };
                            path = parent;
                        }
                    }
                }
                StateTransition::Namespace(transition) => {
                    self.apply_source_namespace(receipt, transition)?;
                }
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
        let from_key = self.selected_head_key(&from)?;
        let cell = self.cells.get(&from_key)?.clone();
        let destination = receipt.input(to)?;
        let selected = self.policy.rename_destination(destination.bytes())?;
        if matches!(cell, AdviceCell::Shadowed | AdviceCell::Deleted) {
            return None;
        }
        if selected.is_empty() {
            self.remove_namespace_import_origin(&from_key);
            self.namespace_imports.remove(&from_key);
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
        self.move_namespace_import_origin(&from_key, &to_key, receipt);
        if let Some(origin) = self.namespace_imports.remove(&from_key) {
            self.namespace_imports.insert(to_key.clone(), origin);
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
            AdviceCell::AuthoredJimClass(class) => {
                AdviceCell::AuthoredJimClass(Arc::new(class.moved(to_key.clone(), receipt)))
            }
            AdviceCell::SourceProcedure(procedure) => {
                AdviceCell::SourceProcedure(Arc::new(procedure.moved(to_key.clone(), receipt)))
            }
            AdviceCell::LogicalSourceProcedure {
                definition,
                mut lineage,
            } => {
                lineage.push(Arc::clone(receipt));
                AdviceCell::LogicalSourceProcedure {
                    definition,
                    lineage,
                }
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
            CommandBindingTransition::Define { name, kind } => {
                let key =
                    self.publication_key(input(name)?.bytes(), AdvicePublicationPurpose::Define)?;
                self.remove_namespace_import_origin(&key);
                self.namespace_imports.remove(&key);
                let cell = if matches!(self.policy, AdviceNamingPolicy::Logical(_))
                    && *kind == tcl_registry::CommandBindingDefinitionKind::Procedure
                {
                    AdviceCell::LogicalSourceProcedure {
                        definition: Arc::clone(receipt),
                        lineage: Vec::new(),
                    }
                } else {
                    AdviceCell::Shadowed
                };
                self.cells.insert(key, cell);
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
                let key = self.selected_head_key(input(name)?.bytes())?;
                self.remove_namespace_import_origin(&key);
                self.namespace_imports.remove(&key);
                self.cells.insert(key, AdviceCell::Deleted);
            }
            CommandBindingTransition::Alias {
                source_interpreter,
                alias,
                target_interpreter,
                target,
                arguments,
                target_lookup,
            } => {
                return self.apply_original_alias(
                    receipt,
                    JimAliasSourceOperands {
                        source_interpreter,
                        alias,
                        target_interpreter,
                        target,
                        arguments,
                        target_lookup: *target_lookup,
                    },
                );
            }
            CommandBindingTransition::Unknown { .. } => return None,
        }
        Some(())
    }
    fn apply_original_alias(
        &mut self,
        receipt: &Arc<OriginalSourceCommandTransition>,
        operands: JimAliasSourceOperands<'_>,
    ) -> Option<()> {
        let JimAliasSourceOperands {
            source_interpreter,
            alias,
            target_interpreter,
            target,
            arguments,
            target_lookup,
        } = operands;
        let input = |subject| receipt.input(subject);

        let current_interpreter = trace_jim_alias_option(
            &receipt.descriptor,
            "jim.alias.source_interpreter",
            receipt.is_current_interpreter(source_interpreter),
        )?;
        if !trace_jim_alias_guard(
            &receipt.descriptor,
            "jim.alias.source_current",
            current_interpreter,
        ) {
            return None;
        }
        let alias =
            trace_jim_alias_option(&receipt.descriptor, "jim.alias.alias_input", input(alias))?;
        let key = trace_jim_alias_option(
            &receipt.descriptor,
            "jim.alias.publication",
            self.publication_key(alias.bytes(), AdvicePublicationPurpose::Alias),
        )?;
        self.remove_namespace_import_origin(&key);
        self.namespace_imports.remove(&key);
        if !trace_jim_alias_guard(
            &receipt.descriptor,
            "jim.alias.namespace",
            self.namespaces.contains(&key.namespace),
        ) || !trace_jim_alias_guard(
            &receipt.descriptor,
            "jim.alias.target_current",
            trace_jim_alias_option(
                &receipt.descriptor,
                "jim.alias.target_interpreter",
                receipt.is_current_interpreter(target_interpreter),
            )?,
        ) {
            // A current alias with foreign target semantics withdraws
            // any earlier current procedure/class meaning at its name.
            self.cells.insert(key, AdviceCell::Shadowed);
            return Some(());
        }
        let prefix = capture_jim_alias_prefix(receipt, arguments)?;
        self.cells.insert(
            key,
            AdviceCell::Alias {
                target: Box::new(
                    trace_jim_alias_option(
                        &receipt.descriptor,
                        "jim.alias.target_input",
                        input(target),
                    )?
                    .clone(),
                ),
                prefix,
                lookup: target_lookup,
                lineage: vec![Arc::clone(receipt)],
            },
        );
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
            StateTransition::Namespace(transition) => {
                use tcl_registry::{NamespaceTransition, NamespaceTransitionTarget};
                let namespace = match transition {
                    NamespaceTransition::Export {
                        namespace,
                        patterns,
                    }
                    | NamespaceTransition::Import {
                        namespace,
                        patterns,
                        ..
                    }
                    | NamespaceTransition::Forget {
                        namespace,
                        patterns,
                    } => {
                        subjects.extend(patterns.iter().cloned());
                        namespace
                    }
                    NamespaceTransition::SetPath { namespace, path } => {
                        subjects.push(path.clone());
                        namespace
                    }
                    NamespaceTransition::Ensemble {
                        namespace,
                        arguments,
                        ..
                    } => {
                        subjects.extend(arguments.iter().cloned());
                        namespace
                    }
                    NamespaceTransition::Delete { namespace } => namespace,
                    _ => continue,
                };
                if let NamespaceTransitionTarget::Named(subject) = namespace {
                    subjects.push(subject.clone());
                }
            }
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
#[derive(Clone, Copy)]
struct JimAliasSourceOperands<'a> {
    source_interpreter: &'a TransitionSubject,
    alias: &'a TransitionSubject,
    target_interpreter: &'a TransitionSubject,
    target: &'a TransitionSubject,
    arguments: &'a [TransitionSubject],
    target_lookup: AliasTargetLookup,
}

fn capture_jim_alias_prefix(
    receipt: &Arc<OriginalSourceCommandTransition>,
    arguments: &[TransitionSubject],
) -> Option<Vec<SourceAdviceWord>> {
    trace_jim_alias_option(
        &receipt.descriptor,
        "jim.alias.prefix",
        arguments
            .iter()
            .enumerate()
            .map(|(ordinal, subject)| {
                let input = trace_jim_alias_option(
                    &receipt.descriptor,
                    "jim.alias.prefix_input",
                    receipt.input(subject),
                )?
                .clone();
                let original = trace_jim_alias_option(
                    &receipt.descriptor,
                    "jim.alias.prefix_word",
                    input.original_word(),
                )?
                .clone();
                Some(SourceAdviceWord {
                    original,
                    value: Some(Arc::from(input.bytes())),
                    input: Some(input),
                    origin: crate::registry_invocation::InvocationWordOrigin::BindingPrefix(
                        ordinal,
                    ),
                })
            })
            .collect::<Option<Vec<_>>>(),
    )
}

// WORK-only, same admission and nonwrapping process quota as the Jim owner trace.
fn trace_jim_alias_option<T>(command: &str, stage: &'static str, value: Option<T>) -> Option<T> {
    // value-transfer-ok: irreducible — selects which calls a debug trace logs.
    // registry-axis-ok: irreducible — selects which calls a debug trace logs; until never
    if command == "alias" {
        crate::registry_invocation::source_structure::trace_jim_source_option(stage, value)
    } else {
        value
    }
}

fn trace_jim_alias_guard(command: &str, stage: &'static str, value: bool) -> bool {
    // value-transfer-ok: irreducible — selects which calls a debug trace logs.
    // registry-axis-ok: irreducible — selects which calls a debug trace logs; until never
    if command == "alias" {
        let _ = crate::registry_invocation::source_structure::trace_jim_source_option(
            stage,
            value.then_some(()),
        );
    }
    value
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
            let argument = trace_jim_alias_option(
                command,
                "jim.alias.capture.argument",
                arguments.get(ordinal),
            )?;
            let input = trace_jim_alias_option(
                command,
                "jim.alias.capture.input",
                argument.input.as_ref(),
            )?
            .clone();
            trace_jim_alias_option(
                command,
                "jim.alias.capture.subject",
                original_transition_subject_matches(&subject, &input, &transitions)
                    .then_some(Some((subject, input))),
            )
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

#[derive(Clone, Copy)]
enum SourceAdviceQuery<'a> {
    NativeDeferredAutoload,
    NormalSourceOutcome(&'a source_point::NormalSourceOutcomeQuery<'a>),
    SourcePoint(&'a source_point::SourcePointQuery<'a>),
    ClassRelation(source_relation_provider::ClassRelationQuery<'a>),
    LogicalClassRelation(source_relation_provider::LogicalClassRelationQuery<'a>),
    LogicalClassRelationCandidates(
        source_relation_provider::LogicalClassRelationCandidatesQuery<'a>,
    ),
    Publication(source_publication_proposal::PublicationProposalQuery<'a>),
}

struct AdviceInvocationContext<'a> {
    origin: &'a Arc<super::SourceOriginId>,
    context: &'a ContextRegistry,
    config: LexerConfig,
    dialect: InvocationDialect,
    policy: &'a AdviceNamingPolicy,
    query: Option<SourceAdviceQuery<'a>>,
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
        logical_scope: Option<
            &Arc<super::source_declared_command::OriginalDeclaredLogicalBodyContext>,
        >,
    ) -> Option<OriginalSourceTransitionAdviceTape> {
        let AdviceInvocation {
            native,
            arguments,
            lineage,
            ..
        } = invocation;
        let mut inventory = OriginalSourceTransitionAdviceTape::default();
        let transitions = schema.state_transitions();
        let ensemble_transition = transitions.facts().iter().any(|fact| {
            matches!(
                &fact.transition,
                StateTransition::Namespace(tcl_registry::NamespaceTransition::Ensemble { .. })
            )
        });
        if ensemble_transition {
            inventory
                .logical_ensemble_transition_receipts
                .insert(native.first()?.span().start(), None);
        }
        let receipt = trace_jim_alias_option(
            command,
            "jim.alias.receipt",
            capture_transition(
                self.origin,
                native,
                command,
                arguments,
                transitions.clone(),
                lineage,
            ),
        );
        if let Some(receipt) = &receipt {
            if ensemble_transition {
                inventory.logical_ensemble_transition_receipts.insert(
                    receipt.original_words().first()?.span().start(),
                    Some(Arc::clone(receipt)),
                );
            }
            let applied =
                trace_jim_alias_guard(command, "jim.alias.applied", graph.apply(receipt).is_some());
            if !applied {
                graph.widen(native);
            }
        }
        let declared = schema.semantics.state_transitions.is_declared();
        let immediate_body = (self
            .authored_roles(schema)
            .0
            .iter()
            .any(|(_, role)| *role == tcl_registry::ArgRole::Body)
            || self.normal_case_source_layout(schema).is_some())
            && !schema
                .semantics
                .traits
                .contains(tcl_registry::Traits::DEFERS_BODY)
            && schema.authored_source_procedure_arguments().is_none();
        if (receipt.is_none() && !transitions.facts().is_empty())
            || transitions.widens(tcl_registry::StateTransitionDomain::CommandBindings)
            || transitions.widens(tcl_registry::StateTransitionDomain::CommandResolution)
        {
            graph.widen(native);
        } else if immediate_body {
            let advice = self.schema_advice(invocation, graph, schema)?;
            let projected =
                self.apply_immediate_body_effects(graph, &advice, schema, logical_scope);
            if !projected.effects_complete {
                graph.widen(native);
            }
            inventory.extend_inventory(projected.inventory);
        } else if !declared
            && !self.normal_source_variable_effects_closed(invocation, schema)
            && !Self::apply_jim_stock_source_effects(graph, invocation, schema)
        {
            graph.widen(native);
        }
        if schema.semantics.procedure_definition.is_none()
            && schema.authored_source_procedure_arguments().is_some()
        {
            Self::apply_source_lookup_residual_effects(graph, invocation, schema);
        }
        Some(inventory)
    }

    fn retain_source_inventory(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        invocation: AdviceInvocation<'_>,
        graph: &AdviceGraph,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        procedure: Option<&source_procedure::OriginalSourceProcedureBody>,
        callback_scope: logical_navigation::LogicalCallbackScope<'_>,
    ) -> Option<()> {
        self.retain_requested_class_relation(invocation, graph, schema);
        self.retain_schema_advice(tape, invocation, graph, schema, procedure.is_some())?;
        self.retain_native_expression_syntax_inventory(tape, graph, invocation, schema);
        self.retain_original_expression_catalogue(tape, graph, invocation, schema);
        self.retain_c_source_lifecycle_inventory(tape, graph, invocation, schema, None, 0);
        self.retain_authored_command_prefix(tape, invocation, graph, schema, None);
        self.retain_definition_member_prefix_inventory(tape, graph, invocation, schema);
        if procedure.is_none() {
            self.retain_original_body_catalogue(tape, graph, invocation, schema);
        }
        self.retain_source_callback_targets(tape, invocation, graph, schema);
        self.retain_logical_callback_candidates(tape, invocation, graph, schema, callback_scope);
        self.retain_logical_name_references(tape, invocation, graph, schema);
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
        procedure_present: bool,
    ) -> Option<()> {
        let retained = graph.native_baseline
            || !invocation.lineage.is_empty()
            || graph.procedure_body.is_some()
            || matches!(self.policy, AdviceNamingPolicy::Logical(_));
        if schema.semantics.procedure_definition.is_some()
            && crate::registry_invocation::source_cut_trace_admitted()
        {
            let image = self.origin.source_image();
            eprintln!(
                "SOURCE_CUT_VALUE_ISSUER origin={:p} data={:p} bytes={} offset={:?} procedure={} native={} lineage={} body={} logical={} retained={retained}",
                Arc::as_ptr(self.origin),
                image.bytes().as_ptr(),
                image.len(),
                invocation.native.first().map(|word| word.span().start()),
                procedure_present,
                graph.native_baseline,
                !invocation.lineage.is_empty(),
                graph.procedure_body.is_some(),
                matches!(self.policy, AdviceNamingPolicy::Logical(_)),
            );
        }
        if retained {
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

    fn retain_arity_debt(
        &self,
        graph: &mut AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> Option<()> {
        if schema.facts().arity_accepts_frozen_arguments() == Some(false) {
            let mut debt = self.schema_advice(invocation, graph, schema)?;
            if !debt
                .obligations
                .contains(&SourceCommandTransitionObligation::UnacceptedSourceArity)
            {
                debt.obligations
                    .push(SourceCommandTransitionObligation::UnacceptedSourceArity);
            }
            if !graph.arity_debts.iter().any(|old| old.as_ref() == &debt) {
                graph.arity_debts.push(Arc::new(debt));
            }
        }
        Some(())
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
        if graph.c_source_namespace.is_some() && !graph.native_baseline {
            obligations.push(SourceCommandTransitionObligation::ConditionalCNamespaceApplicability);
        }
        let c_source_namespace = (!graph.native_baseline)
            .then(|| graph.c_source_lookup_namespace().cloned())
            .flatten();
        let c_source_path = c_source_namespace.as_ref().and_then(|namespace| {
            (!graph.unknown_paths.contains(namespace)).then(|| {
                graph
                    .namespace_paths
                    .get(namespace)
                    .cloned()
                    .unwrap_or_default()
            })
        });
        if c_source_path.as_ref().is_some_and(|path| {
            path.iter()
                .any(|namespace| !graph.namespaces.contains(namespace))
        }) {
            obligations.push(
                SourceCommandTransitionObligation::UnretainedCSourcePathProviderApplicability,
            );
        }
        if !graph.uncertain_operations.is_empty() {
            obligations.push(SourceCommandTransitionObligation::UnknownEarlierMutation);
        }
        if !graph.arity_debts.is_empty() {
            obligations.push(SourceCommandTransitionObligation::UnacceptedSourceArity);
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
            logical_scope: None,
            c_source_body: None,
            c_source_namespace,
            c_source_path,
            procedure_body: graph.procedure_body.clone(),
            member_catalogue_body: None,
            catalogue_body: None,
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

    fn observe_publication_proposal(&self, graph: &AdviceGraph) {
        if let Some(SourceAdviceQuery::Publication(query)) = self.query {
            query.observe(graph);
        }
    }

    fn withdraw_missing_publication_phase(
        &self,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) {
        let Some(SourceAdviceQuery::Publication(query)) = self.query else {
            return;
        };
        let (roles, complete) = self.authored_roles(schema);
        let relevant = !complete
            || roles
                .iter()
                .any(|(_, role)| *role == tcl_registry::ArgRole::Expr)
            || schema
                .authored_source_script_arguments()
                .is_none_or(|arguments| !arguments.is_empty());
        if relevant {
            query.withdraw();
        }
    }

    fn partial_expression_geometry(
        &self,
        tail: tcl_lexer::NativeScriptWordCut,
    ) -> Option<(
        tcl_lexer::NativeScriptWordCut,
        tcl_lexer::SourceSegmentationPurpose,
    )> {
        let image = self.origin.source_image();
        if tail.cut.kind != tcl_lexer::ParseIssueKind::MissingBrace
            || image.bytes().get(tail.cut.term as usize) != Some(&b'{')
        {
            return None;
        }
        if tail.cut.offset as usize == image.bytes().len() {
            Some((tail, tcl_lexer::SourceSegmentationPurpose::WholeInput))
        } else {
            let bracket = tail.cut.offset;
            if image.bytes().get(bracket as usize) != Some(&b'[') {
                return None;
            }
            let outer = tcl_lexer::native_script_words_in(
                image.clone(),
                tcl_lexer::Span::new(tail.command_start, bracket),
                self.config,
            )
            .ok()?;
            let [parent] = outer.commands.as_slice() else {
                return None;
            };
            let last = parent.words.last()?;
            if outer.fatal_tail.is_some()
                || !image
                    .bytes()
                    .get(last.span().end() as usize..bracket as usize)?
                    .iter()
                    .all(u8::is_ascii_whitespace)
                || last.span().end() == bracket
                || parent.words.iter().any(|word| {
                    word.group().expand
                        || tcl_syntax::word_rules::original_static_word_unicode_value(word)
                            .is_none()
                })
            {
                return None;
            }
            let child = tcl_lexer::native_script_words_in_with_purpose(
                image.clone(),
                tcl_lexer::Span::new(bracket + 1, image.bytes().len().try_into().ok()?),
                self.config,
                tcl_lexer::SourceSegmentationPurpose::NestedScript,
            )
            .ok()?;
            if !child.commands.is_empty() {
                return None;
            }
            let child_tail = child.fatal_tail?;
            if child_tail.cut.kind != tcl_lexer::ParseIssueKind::MissingBrace
                || child_tail.cut.offset as usize != image.bytes().len()
                || child_tail.cut.term != tail.cut.term
            {
                return None;
            }
            Some((
                child_tail,
                tcl_lexer::SourceSegmentationPurpose::NestedScript,
            ))
        }
    }

    fn partial_expression_span(
        &self,
        graph: &AdviceGraph,
        tail: tcl_lexer::NativeScriptWordCut,
    ) -> Option<tcl_lexer::Span> {
        if !graph.uncertain_operations.is_empty() || !graph.arity_debts.is_empty() {
            return None;
        }
        let image = self.origin.source_image();
        let (tail, purpose) = self.partial_expression_geometry(tail)?;
        let prefix = tcl_lexer::native_script_words_in_with_purpose(
            image.clone(),
            tcl_lexer::Span::new(tail.command_start, tail.cut.term),
            self.config,
            purpose,
        )
        .ok()?;
        let [command] = prefix.commands.as_slice() else {
            return None;
        };
        let [word] = command.words.as_slice() else {
            return None;
        };
        if prefix.fatal_tail.is_some()
            || !image
                .bytes()
                .get(word.span().end() as usize..tail.cut.term as usize)?
                .iter()
                .all(u8::is_ascii_whitespace)
        {
            return None;
        }
        let written = original_words(&command.words, self.policy)?;
        let head = written.first()?.input.as_ref()?;
        if graph.blocks_registry_source(head) {
            return None;
        }
        let (name, arguments, lineage) = graph.resolve(head)?;
        if !arguments.is_empty() || !lineage.is_empty() {
            return None;
        }
        let args = [tcl_registry::InvocationWord::Dynamic];
        let resolution =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                self.context.commands(),
                Some(self.context.context()),
                tcl_registry::InvocationWords::structured(
                    tcl_registry::InvocationWord::Literal(&name),
                    &args,
                )
                .with_dialect(self.dialect),
                tcl_dialect::model::InvocationRealm::RuleLoader,
            );
        let schema = resolution.resolved()?;
        if schema.facts().argument_offset != 0 {
            return None;
        }
        let (roles, complete) = self.authored_roles(&schema);
        (complete
            && roles.iter().filter(|(index, _)| *index == 0).count() == 1
            && roles.contains(&(0, tcl_registry::ArgRole::Expr)))
        .then(|| tcl_lexer::Span::new(tail.cut.term + 1, tail.cut.offset))
    }

    fn retain_original_invocation(
        &self,
        graph: &mut AdviceGraph,
        tape: &mut OriginalSourceTransitionAdviceTape,
        invocation: &tcl_lexer::NativeScriptCommandWords,
        callback_scope: logical_navigation::LogicalCallbackScope<'_>,
    ) -> Option<()> {
        self.begin_normal_source_invocation();
        self.observe_publication_proposal(graph);
        tape.retain_arity_prefix(graph, &invocation.words);
        let retained =
            self.retain_original_invocation_inner(graph, tape, invocation, callback_scope);
        self.observe_publication_proposal(graph);
        retained
    }

    fn retain_original_invocation_prefix(
        &self,
        graph: &mut AdviceGraph,
        tape: &mut OriginalSourceTransitionAdviceTape,
        invocation: &tcl_lexer::NativeScriptCommandWords,
        written: &[SourceAdviceWord],
    ) -> bool {
        self.retain_native_procedure_may_command(tape, graph, &invocation.words, written);
        self.retain_original_operand_constructor_calls(tape, graph, &invocation.words, None);
        self.retain_original_source_receiver(tape, graph, &invocation.words, written)
    }

    fn retain_unresolved_invocation_operands(
        &self,
        graph: &mut AdviceGraph,
        tape: &mut OriginalSourceTransitionAdviceTape,
        invocation: &tcl_lexer::NativeScriptCommandWords,
    ) -> Option<()> {
        if matches!(self.policy, AdviceNamingPolicy::Logical(_)) {
            // The original children execute before an unknown parent head;
            // retaining them does not select that head from result data.
            let _ = self.inspect_operand_effects(graph, &invocation.words, tape);
            tape.registry_barriers
                .insert(invocation.words.first()?.span().start());
        }
        graph.widen(&invocation.words);
        Some(())
    }

    fn append_original_invocation_operands(
        arguments: &mut Vec<SourceAdviceWord>,
        written: Vec<SourceAdviceWord>,
    ) {
        for (ordinal, argument) in arguments.iter_mut().enumerate() {
            argument.origin =
                crate::registry_invocation::InvocationWordOrigin::BindingPrefix(ordinal);
        }
        arguments.extend(written.into_iter().skip(1));
    }

    fn retain_invocation_barrier(
        tape: &mut OriginalSourceTransitionAdviceTape,
        invocation: &tcl_lexer::NativeScriptCommandWords,
        original_barrier: bool,
    ) -> Option<()> {
        if original_barrier {
            tape.registry_barriers
                .insert(invocation.words.first()?.span().start());
        }
        Some(())
    }

    fn retain_original_invocation_inner(
        &self,
        graph: &mut AdviceGraph,
        tape: &mut OriginalSourceTransitionAdviceTape,
        invocation: &tcl_lexer::NativeScriptCommandWords,
        callback_scope: logical_navigation::LogicalCallbackScope<'_>,
    ) -> Option<()> {
        tape.represented
            .insert(invocation.words.first()?.span().start());
        let Some(written) = original_words(&invocation.words, self.policy) else {
            graph.widen(&invocation.words);
            return Some(());
        };
        if self.retain_original_invocation_prefix(graph, tape, invocation, &written) {
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
            self.retain_unresolved_invocation_operands(graph, tape, invocation)?;
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
        Self::retain_invocation_barrier(tape, invocation, original_barrier)?;
        OriginalSourceTransitionAdviceTape::retain_logical_procedure_call(
            tape,
            graph,
            &invocation.words,
        );
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
        Self::append_original_invocation_operands(&mut arguments, written);
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
        let mut procedure = None;
        self.retain_original_invocation_inventory(
            graph,
            tape,
            selected,
            &schema,
            callback_scope,
            &mut procedure,
        )?;
        self.retain_interpreter_paths(tape, selected, graph, &schema);
        let factory = self.registered_factory(graph, selected, &schema);
        let authored_jim = self.authored_jim_class_source(graph, selected, &schema, None);
        let class = self.retain_source_class_declaration(tape, graph, selected, &schema);
        self.retain_source_configured_class(tape, graph, selected, &schema);
        self.retain_original_class_operand_references(
            tape,
            graph,
            class.as_ref(),
            procedure.as_ref(),
        );
        let body_inventory =
            self.apply_schema_transition(graph, selected, &command, &schema, None)?;
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
        self.retain_authored_jim_procedure_adoptions(tape, graph, authored_jim.as_ref());
        Self::install_authored_jim_class(tape, graph, authored_jim);
        source_procedure::install_original_procedure(graph, procedure);
        Some(())
    }
    fn retain_original_invocation_inventory(
        &self,
        graph: &mut AdviceGraph,
        tape: &mut OriginalSourceTransitionAdviceTape,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        callback_scope: logical_navigation::LogicalCallbackScope<'_>,
        procedure: &mut Option<Arc<source_procedure::OriginalSourceProcedureBody>>,
    ) -> Option<()> {
        self.retain_arity_debt(graph, invocation, schema)?;
        self.observe_source_point(graph, invocation.native, schema);
        self.observe_normal_source_operation(graph, schema);
        *procedure = self.source_procedure_declaration(graph, invocation, schema);
        if let Some(shape) = schema.authored_source_procedure_arguments() {
            source_class::merge_receipt(
                &mut tape.conditional_procedure_declarations,
                invocation.native.first()?.span().start(),
                procedure.clone().map(|procedure| (procedure, shape)),
            );
        }
        self.retain_source_inventory(
            tape,
            invocation,
            graph,
            schema,
            procedure.as_deref(),
            callback_scope,
        )?;
        Some(())
    }
}

impl super::SourceCommandBindings {
    /// Ordinary selected Registry-schema arity debts in this exact source prefix.
    /// Receiver, registered and source-header alternatives retain independent
    /// owners. An empty list proves no complete invocation acceptance, reached
    /// lookup, successful call or Normal result.
    pub(crate) fn original_source_prefix_arity_debts(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        original: &[NativeWord],
    ) -> Option<Vec<Arc<OriginalSourceCommandTransitionAdvice>>> {
        if self.source_metadata_owner().source_analysis_input() != Some(input) {
            return None;
        }
        let origin = self.source_origin()?;
        if original.is_empty()
            || original.iter().any(|word| {
                word.image() != origin.source_image() || word.config() != input.lexer_config()
            })
            || !self.matches_original_source_image(origin.source_image(), input.lexer_config())
        {
            return None;
        }
        let tape = self.original_source_transition_advice_tape_for(
            input.borrowed_context_registry(),
            OriginalSourceCataloguePurpose::CompletePrefix,
        )?;
        let prefix = tape
            .arity_prefixes
            .get(&original[0].span().start())?
            .as_ref()?;
        if prefix.original.as_ref() != original
            || prefix.debts.iter().any(|debt| {
                !Arc::ptr_eq(&debt.site.source, origin)
                    || !debt.matches_source(origin.source_image(), input.lexer_config())
                    || !debt.matches_context(input.borrowed_context_registry())
                    || !debt
                        .obligations
                        .contains(&SourceCommandTransitionObligation::UnacceptedSourceArity)
            })
        {
            return None;
        }
        Some(prefix.debts.clone())
    }

    pub(crate) const fn original_source_lexer_config(&self) -> Option<LexerConfig> {
        self.lexer_config
    }

    pub(crate) fn original_logical_source_name_advice_input(
        &self,
    ) -> Option<&crate::analyser::ResolvedAnalysisInput> {
        self.final_state.logical_source_name_advice_input()
    }

    #[cfg(test)]
    fn prepare_source_advice_tape(
        &self,
        context: &ContextRegistry,
    ) -> Option<AdviceTapeInputs<'_>> {
        let seed = self.original_source_transition_seed()?;
        let prepared = seed
            .prepare_source_advice_tape(context, OriginalSourceCataloguePurpose::CompleteProgram)?;
        Some(AdviceTapeInputs {
            origin: self.source_origin()?,
            config: prepared.config,
            policy: prepared.policy,
            dialect: prepared.dialect,
            plan: prepared.plan,
        })
    }

    #[cfg(test)]
    fn initial_source_advice_graph(
        &self,
        context: &ContextRegistry,
        policy: &AdviceNamingPolicy,
    ) -> Option<AdviceGraph> {
        self.original_source_transition_seed()?
            .initial_source_advice_graph(context, policy)
    }

    pub(super) fn original_source_transition_seed(&self) -> Option<OriginalSourceTransitionSeed> {
        // An unentered interpretation offers no advice to re-interpret from.
        if self.unentered {
            return None;
        }
        if self.final_state.baseline.source_point_entry.is_some()
            && self.original_inherited_source_entry().is_none()
        {
            return None;
        }
        let origin = self.source_origin()?;
        let config = self.lexer_config?;
        self.matches_original_source_image(origin.source_image(), config)
            .then_some(())?;
        Some(OriginalSourceTransitionSeed {
            analysis_owner: self.original_analysis_owner().cloned(),
            origin: Arc::clone(origin),
            config,
            baseline: Arc::clone(&self.final_state.baseline),
            logical_input: self.final_state.logical_source_name_advice_input().cloned(),
            stock_source_effects: self.original_jim_stock_source_effects(),
        })
    }

    /// Independently interpreted authored root-script transitions. This owner
    /// never mutates, publishes or borrows the retained runtime command state.
    pub(crate) fn original_logical_procedure_body_cards(
        &self,
    ) -> Option<Vec<Arc<OriginalLogicalProcedureBody>>> {
        let seed = self.original_source_transition_seed()?;
        if seed.baseline.native_entry.is_some() {
            return None;
        }
        let input = seed.logical_input.as_ref()?;
        input.has_logical_source_name_context().then_some(())?;
        let context = seed.source_context(input.borrowed_context_registry().commands())?;
        let tape = seed.prepare_for(&context, OriginalSourceCataloguePurpose::CompletePrefix)?;
        Some(
            tape.logical_procedure_bodies
                .into_values()
                .flatten()
                .collect(),
        )
    }

    /// Exact deferred Logical command scope for MAY body dependencies only.
    pub(crate) fn original_logical_summary_scope(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        original: &[NativeWord],
    ) -> Option<Arc<super::OriginalDeclaredLogicalBodyContext>> {
        if self.original_logical_source_name_advice_input() != Some(input)
            || self.source_metadata_owner().source_analysis_input() != Some(input)
        {
            return None;
        }
        let tape =
            self.original_source_transition_advice_tape(input.borrowed_context_registry())?;
        let record = tape
            .logical_summary_scopes
            .get(&original.first()?.span().start())?
            .as_ref()?;
        (record.original.as_ref() == original
            && record.scope.matches_input(input)
            && record.scope.owns_words(original))
        .then(|| Arc::clone(&record.scope))
    }

    pub(crate) fn original_source_transition_advice_tape(
        &self,
        context: &ContextRegistry,
    ) -> Option<OriginalSourceTransitionAdviceTape> {
        self.original_source_transition_seed()?.prepare(context)
    }

    pub(crate) fn original_normal_source_procedure_advice_tape(
        &self,
        context: &ContextRegistry,
    ) -> Option<OriginalSourceTransitionAdviceTape> {
        self.original_source_transition_seed()?
            .normal_source_outcome_advice(context, self)
            .map(|(_, tape)| tape)
    }

    pub(crate) fn original_source_transition_advice_tape_for(
        &self,
        context: &ContextRegistry,
        purpose: OriginalSourceCataloguePurpose,
    ) -> Option<OriginalSourceTransitionAdviceTape> {
        self.original_source_transition_seed()?
            .prepare_for(context, purpose)
    }

    /// [`Self::original_source_transition_advice_tape`], shared with every
    /// other realm and bindings copy that holds an equal seed.
    pub(crate) fn original_source_transition_advice_tape_shared(
        &self,
        context: &ContextRegistry,
    ) -> Arc<Option<OriginalSourceTransitionAdviceTape>> {
        self.original_source_transition_advice_tape_for_shared(
            context,
            OriginalSourceCataloguePurpose::CompleteProgram,
        )
    }

    /// [`Self::original_source_transition_advice_tape_for`], shared.
    pub(crate) fn original_source_transition_advice_tape_for_shared(
        &self,
        context: &ContextRegistry,
        purpose: OriginalSourceCataloguePurpose,
    ) -> Arc<Option<OriginalSourceTransitionAdviceTape>> {
        match self.original_source_transition_seed() {
            Some(seed) => seed.prepared_shared(context, purpose),
            None => Arc::new(None),
        }
    }
}

impl OriginalSourceTransitionSeed {
    /// A fresh tape for the realm caches. Not shared across realms: receipts
    /// derived from a tape are compared by identity, and a process-wide memo
    /// made those comparisons depend on what was analysed earlier.
    pub(super) fn prepared_shared(
        &self,
        context: &ContextRegistry,
        purpose: OriginalSourceCataloguePurpose,
    ) -> Arc<Option<OriginalSourceTransitionAdviceTape>> {
        Arc::new(self.prepare_for(context, purpose))
    }
}

/// Immutable whole-source issuer shared by independently requested purposes.
/// The baseline retains its real metadata and entry; no body becomes a new entry.
#[derive(Debug, Clone)]
pub(super) struct OriginalSourceTransitionSeed {
    // Only the Normal-source query uses this attachment. Source syntax reuse
    // keeps its existing four-field seed equality below.
    pub(super) analysis_owner: Option<Arc<super::OriginalSourceAnalysisOwner>>,
    pub(super) origin: Arc<super::SourceOriginId>,
    pub(super) config: LexerConfig,
    pub(super) baseline: Arc<super::BindingBaseline>,
    pub(super) logical_input: Option<crate::analyser::ResolvedAnalysisInput>,
    pub(super) stock_source_effects: Vec<super::jim_stock_class::OriginalJimStockSourceEffects>,
}
impl PartialEq for OriginalSourceTransitionSeed {
    fn eq(&self, other: &Self) -> bool {
        self.origin == other.origin
            && self.config == other.config
            && self.baseline == other.baseline
            && self.logical_input == other.logical_input
            && self.stock_source_effects == other.stock_source_effects
    }
}
impl Eq for OriginalSourceTransitionSeed {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OriginalSourceCataloguePurpose {
    CompleteProgram,
    /// Authentic complete commands before the separately retained parse cut.
    CompletePrefix,
    /// Exact sealed region plan, without whole-image coverage or absence.
    ConditionalRegion,
}

impl OriginalSourceTransitionSeed {
    pub(super) fn matches_registry(&self, registry: &tcl_registry::CommandRegistry) -> bool {
        self.baseline.registry_snapshot.as_ref() == Some(&registry.snapshot().semantic_key())
    }

    pub(super) fn source_context(
        &self,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Arc<ContextRegistry>> {
        if !self.matches_registry(registry) {
            return None;
        }
        match &self.baseline.metadata_context {
            crate::registry_invocation::OwnedInvocationMetadataContext::SuppliedSource(input) => {
                crate::registry_invocation::InvocationMetadataContext::for_source_input(
                    registry,
                    input,
                    self.config,
                    Some(input.unit_profile()),
                )?;
                Some(input.context_registry())
            }
            crate::registry_invocation::OwnedInvocationMetadataContext::Standalone => {
                let profile = registry.profile()?;
                // Explicit scalar compatibility selects static availability only.
                // The exact frozen store keeps every override and generation.
                Some(Arc::new(
                    tcl_registry::model::ingress::context_for_profile(profile)
                        .with_command_store(registry.snapshot().shared_registry()),
                ))
            }
            crate::registry_invocation::OwnedInvocationMetadataContext::Supplied(_)
            | crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable => None,
        }
    }

    fn prepare_source_advice_tape(
        &self,
        context: &ContextRegistry,
        purpose: OriginalSourceCataloguePurpose,
    ) -> Option<AdviceTapeInputs<'_>> {
        let origin = &self.origin;
        let config = self.config;
        let baseline = &self.baseline;
        let policy = if let Some(native) = baseline
            .execution_name_policy
            .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
        {
            AdviceNamingPolicy::Native(native)
        } else {
            AdviceNamingPolicy::Logical(self.logical_input.as_ref()?.clone())
        };
        let dialect = baseline.dialect?;
        if baseline.registry_snapshot.as_ref()
            != Some(&context.commands().snapshot().semantic_key())
        {
            return None;
        }
        if purpose == OriginalSourceCataloguePurpose::ConditionalRegion
            && baseline
                .source_point_entry
                .as_ref()
                .is_none_or(|entry| entry.conditional_region().is_none())
        {
            return None;
        }
        let plan = match baseline
            .source_point_entry
            .as_ref()
            .filter(|entry| entry.conditional_region().is_some())
        {
            Some(entry) => {
                // Region catalogues are positive prefix views only. They may
                // never certify a whole-file inventory or missing occurrence.
                if purpose == OriginalSourceCataloguePurpose::CompleteProgram {
                    return None;
                }
                entry.original_region_plan(origin, config)?
            }
            None => origin
                .original_script_words_in(
                    tcl_lexer::Span::new(
                        0,
                        u32::try_from(origin.source_image().bytes().len()).ok()?,
                    ),
                    config,
                    tcl_lexer::SourceSegmentationPurpose::WholeInput,
                )
                .ok()?,
        };
        if plan.fatal_tail.is_some() && purpose == OriginalSourceCataloguePurpose::CompleteProgram {
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
        let baseline = &self.baseline;
        if let Some(entry) = &baseline.source_point_entry {
            return entry.source_advice_graph(context)?.child_graph(policy);
        }
        let dialect = baseline.dialect?;
        let seed = AdviceCatalogueSeedMemo::shared().get(context, policy, dialect)?;
        let mut graph = AdviceGraph::from_catalogue_seed(policy, dialect, &seed);
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

    pub(super) fn prepare(
        &self,
        context: &ContextRegistry,
    ) -> Option<OriginalSourceTransitionAdviceTape> {
        self.prepare_for(context, OriginalSourceCataloguePurpose::CompleteProgram)
    }

    pub(super) fn prepare_for(
        &self,
        context: &ContextRegistry,
        purpose: OriginalSourceCataloguePurpose,
    ) -> Option<OriginalSourceTransitionAdviceTape> {
        self.prepare_for_query(context, purpose, None)
    }

    fn prepare_for_query(
        &self,
        context: &ContextRegistry,
        purpose: OriginalSourceCataloguePurpose,
        query: Option<SourceAdviceQuery<'_>>,
    ) -> Option<OriginalSourceTransitionAdviceTape> {
        let AdviceTapeInputs {
            origin,
            config,
            policy,
            dialect,
            plan,
        } = self.prepare_source_advice_tape(context, purpose)?;
        let mut graph = self.initial_source_advice_graph(context, &policy)?;
        graph.stock_source_effects = self
            .stock_source_effects
            .iter()
            .filter(|effect| {
                effect.matches_seed(self.analysis_owner.as_ref(), origin, &self.baseline)
                    && plan
                        .commands
                        .iter()
                        .any(|command| command.words == effect.original())
            })
            .cloned()
            .collect();
        let mut tape = OriginalSourceTransitionAdviceTape::default();
        let partial_tail = plan.fatal_tail;
        if matches!(policy, AdviceNamingPolicy::Logical(_))
            && let Some(tail) = plan.fatal_tail
        {
            // CompletePrefix retains positive source cards, but its malformed
            // remainder cannot certify a whole-document catalogue or emptiness.
            tape.logical_procedure_catalogue_refusals.insert(
                LogicalProcedureCatalogueRefusal::RootLexicalTailUnavailable(tail.command_start),
            );
        }
        let advice_context = AdviceInvocationContext {
            origin,
            context,
            config,
            dialect,
            policy: &policy,
            query,
        };
        advice_context.observe_publication_proposal(&graph);
        for invocation in plan.commands {
            // A cooperative stop point only: a cancelled host read unwinds here.
            crate::analysis_control::unwind_point(
                crate::analysis_control::AnalysisCheckpoint::Body,
            );
            if let AdviceNamingPolicy::Logical(input) = &policy
                && self.baseline.metadata_context.source_analysis_input() == Some(input)
                && self.baseline.logical_source_input.as_ref() == Some(input)
                && self.baseline.native_entry.is_none()
                && self.baseline.hosted_execution_context.is_none()
                && self.baseline.vendor_source_input.is_none()
                && !self.baseline.unknown_entry
            {
                // Only the whole source seed's original root command traversal
                // supplies this conditional Document recipe. Nested bodies keep
                // their independent source scope, and no runtime frame follows.
                tape.retain_logical_document_navigation(&graph, &invocation.words, origin);
            }
            advice_context.retain_original_invocation(
                &mut graph,
                &mut tape,
                &invocation,
                logical_navigation::LogicalCallbackScope::Document,
            )?;
        }
        if purpose == OriginalSourceCataloguePurpose::CompletePrefix
            && self.baseline.source_point_entry.is_none()
            && self.baseline.hosted_execution_context.is_none()
            && self.baseline.vendor_source_input.is_none()
            && !self.baseline.unknown_entry
            && let Some(tail) = partial_tail
        {
            tape.partial_expression_span = advice_context.partial_expression_span(&graph, tail);
        }
        advice_context.finish_normal_source_graph(&graph);
        advice_context.retain_future_original_procedure_bodies(&mut tape, &graph);
        advice_context.retain_final_original_procedure_publications(&mut tape, &graph);
        advice_context.retain_final_original_class_publications(&mut tape, &graph);
        advice_context.retain_final_class_display_publications(&mut tape, &graph);
        Some(tape)
    }
}

/// Original declaration source selected by the existing transition interpreter.
/// This receipt supplies no current command target, publication or entered frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OriginalConditionalProcedureDeclaration {
    pub(super) advice: Arc<OriginalSourceCommandTransitionAdvice>,
    pub(super) body: Arc<crate::registry_invocation::OriginalSourceScriptBody>,
    pub(super) name: NativeWord,
    pub(super) parameters: NativeWord,
    pub(super) namespace: ByteNamespacePath,
    pub(super) label: String,
}

/// Reborrow an immutable original source observation, never its withdrawn
/// execution policy or frame. The ordinary layout/table admission is unchanged.
pub(super) fn original_conditional_procedure_declaration(
    site: &super::CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    row: &super::declaration_layout::DeclarationLayoutObservation,
    owner: &Arc<super::OriginalSourceAnalysisOwner>,
    registry: &tcl_registry::CommandRegistry,
) -> Option<OriginalConditionalProcedureDeclaration> {
    let baseline = &row.snapshot.state.baseline;
    if !Arc::ptr_eq(&owner.source, &site.source)
        || owner.config != row.config
        || !owner.metadata.is_standalone()
        || !baseline.metadata_context.is_standalone()
        || baseline.native_entry.is_some()
        || baseline.logical_source_input.is_some()
        || baseline.vendor_source_input.is_some()
        || baseline.hosted_execution_context.is_some()
        || baseline.unknown_entry
        || row.words.as_ref() != tokens.words()
        || row.snapshot.state.current_source_origin.as_ref() != Some(&site.source)
        || !row.entry.owns_source(&site.source, site.offset)
        || !matches!(&row.namespace, super::SourceNamespaceKey::Authored(namespace) if namespace == "::")
        || baseline.registry_owner.as_ref()?.semantic_key() != registry.snapshot().semantic_key()
    {
        return None;
    }
    // This bounded producer owns the original root script, not a reconstructed
    // child activation or the current (possibly Unknown) execution frame.
    let super::declaration_layout::OriginalDiagnosticFrameEntry::RootScript {
        source,
        namespace,
        ..
    } = row.entry.as_ref()
    else {
        return None;
    };
    if !Arc::ptr_eq(&source.origin, &site.source)
        || source.base() != 0
        || source.text.bytes() != site.source.source_image().bytes()
        || namespace != &row.namespace
    {
        return None;
    }
    let original = site
        .source
        .original_lexical_command(tokens.words(), site.offset, row.config)?;
    let seed = OriginalSourceTransitionSeed {
        analysis_owner: Some(Arc::clone(owner)),
        origin: Arc::clone(&site.source),
        config: row.config,
        baseline: Arc::clone(baseline),
        logical_input: None,
        stock_source_effects: Vec::new(),
    };
    let context = seed.source_context(registry)?;
    let selections = conditional_procedure_declaration_selections(owner, &seed, &context)?;
    let (declaration, shape) = selections.get(&site.offset)?.as_ref()?;
    let advice = &declaration.declaration;
    if advice.site() != site
        || advice.original_words() != original.native_words()
        || original
            .native_words()
            .iter()
            .any(|word| word.group().expand)
        || !advice.matches_source(site.source.source_image(), row.config)
        || !advice.matches_context(&context)
        || advice.uncertain_operations().is_empty()
        || !declaration
            .body
            .matches_source(site.source.source_image(), row.config)
        || !declaration.body.matches_context(&context)
    {
        return None;
    }
    let parameter = advice.arguments.get(shape.parameters)?;
    let name = advice.arguments.get(shape.name)?;
    let body = advice.arguments.get(shape.body)?;
    if [name, parameter, body].iter().any(|word| {
        !matches!(
            word.origin,
            crate::registry_invocation::InvocationWordOrigin::Written(_)
        )
    }) || Some(&name.original) != declaration.name.original_word()
        || &body.original != declaration.body.original_container()
    {
        return None;
    }
    Some(OriginalConditionalProcedureDeclaration {
        advice: Arc::clone(advice),
        body: Arc::clone(&declaration.body),
        name: declaration.name.original_word()?.clone(),
        parameters: parameter.original.clone(),
        namespace: declaration.declaration_slot.namespace.clone(),
        label: tcl_syntax::naming::authored_source_command_spelling(&declaration.declaration_slot)?,
    })
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod possible_occupant_alias_tests;

#[cfg(test)]
mod advice_cell_storage_tests;

#[cfg(test)]
mod declaration_value_controls;
