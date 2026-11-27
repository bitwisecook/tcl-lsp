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

use crate::signature_scan::scope::SignatureSourceNameInput;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;
use tcl_core_types::{ByteCommandSlot, ByteNamespacePath, NameBytes};
use tcl_lexer::{LexerConfig, NativeWord, SourceImage};
use tcl_native_model::InvocationDialect;
use tcl_registry::model::{ContextRegistry, ResolvedContext};
use tcl_registry::{AliasTargetLookup, TransitionSubject};
use tcl_syntax::naming::{NamePolicyProtocol, NativeNameProtocol};

mod advice_graph;
mod advice_tape;
mod command_advice;
mod conditional_declarations;
mod invocation_advice;
mod tape_preparation;
mod transition_capture;

use advice_graph::{lookup_key, root_context};
pub(crate) use advice_tape::OriginalSourceTransitionAdviceLookup;
use conditional_declarations::ConditionalProcedureDeclarationSelections;
#[cfg(test)]
use conditional_declarations::conditional_procedure_declaration_selections;
pub(super) use conditional_declarations::{
    OriginalConditionalProcedureDeclaration, original_conditional_procedure_declaration,
};
pub(crate) use tape_preparation::OriginalSourceCataloguePurpose;
pub(super) use tape_preparation::OriginalSourceTransitionSeed;
#[cfg(test)]
use transition_capture::original_transition_subject_matches;
use transition_capture::{capture_transition, original_words, registry_words};

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

/// Derived source selections only; the existing analysis allocation remains
/// the owner. Weak baseline identity avoids retaining a baseline/owner cycle.
#[derive(Debug, Clone)]
pub(super) struct OriginalConditionalProcedureDeclarationMemo {
    baseline: std::sync::Weak<super::BindingBaseline>,
    registry: tcl_registry::RegistrySemanticKey,
    selections: Arc<ConditionalProcedureDeclarationSelections>,
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

struct SourceAliasCellSelection<'a> {
    cell: &'a AdviceCell,
    slot: ByteCommandSlot,
    prefix: Vec<SourceAdviceWord>,
    lineage: Vec<Arc<OriginalSourceCommandTransition>>,
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

#[cfg(test)]
mod tests;

#[cfg(test)]
mod possible_occupant_alias_tests;

#[cfg(test)]
mod advice_cell_storage_tests;

#[cfg(test)]
mod declaration_value_controls;
