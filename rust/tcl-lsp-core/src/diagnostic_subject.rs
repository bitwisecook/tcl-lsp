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

//! Versioned diagnostic subject transport for LSP, CLI and MCP consumers.
//!
//! These descriptions preserve bytes and reporting coordinates. Deserialising
//! one cannot recreate an original word, invocation, native issuer or edit grant.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tcl_compiler::analyser::{Diagnostic, DiagnosticSubject};
use tcl_syntax::naming::{NamePolicyAuthority, NamePolicyProtocol, NativeNameProtocol};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PolicyDescription {
    engine: String,
    version: String,
    authority: String,
}

impl From<NamePolicyProtocol> for PolicyDescription {
    fn from(policy: NamePolicyProtocol) -> Self {
        let (engine, version) = match policy.recipe() {
            NativeNameProtocol::C(version) => ("c-tcl", version.version_string()),
            NativeNameProtocol::Jim084 => ("jim", "0.84"),
        };
        Self {
            engine: engine.to_owned(),
            version: version.to_owned(),
            authority: match policy.authority() {
                NamePolicyAuthority::Native => "native",
                NamePolicyAuthority::AuthoredSimulation => "authored",
            }
            .to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum MathDispatchDescription {
    FixedTable,
    CommandTable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum MathClassificationDescription {
    Absent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum VisibilityObligationDescription {
    SuccessfulSourceOperations,
    RegistryInitialization,
    NoUnmodelledInterpreterMutation,
    NoObserverInterference,
    SourceBodyEntered,
    OriginalOperandMaterialization,
    UnknownExecutingReceiverNamespace,
    NoEarlierReceiverNamespaceBinding,
}
impl From<tcl_compiler::analyser::InterpreterVisibilityObligation>
    for VisibilityObligationDescription
{
    fn from(obligation: tcl_compiler::analyser::InterpreterVisibilityObligation) -> Self {
        use tcl_compiler::analyser::InterpreterVisibilityObligation as Original;
        match obligation {
            Original::SuccessfulSourceOperations => Self::SuccessfulSourceOperations,
            Original::RegistryInitialization => Self::RegistryInitialization,
            Original::NoUnmodelledInterpreterMutation => Self::NoUnmodelledInterpreterMutation,
            Original::NoObserverInterference => Self::NoObserverInterference,
            Original::SourceBodyEntered => Self::SourceBodyEntered,
            Original::OriginalOperandMaterialization => Self::OriginalOperandMaterialization,
            Original::UnknownExecutingReceiverNamespace => Self::UnknownExecutingReceiverNamespace,
            Original::NoEarlierReceiverNamespaceBinding => Self::NoEarlierReceiverNamespaceBinding,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum RegistryPurposeDescription {
    ClauseShape,
    ContextGate,
    ErrorCapture,
    LiteralArgument,
    VariableName,
    OptionTerminator,
    DisabledOption,
    Subcommand,
    DeprecatedCommand,
    Expression,
    Arity,
    ArgumentRelationship,
    Lifecycle,
    ChannelArgument,
    PackageRequirement,
    PackageOrdering,
    AppendList,
    CaseBody,
    OptionOnly,
    PatternSubstitution,
    IndexBounds,
    /// Literal shape advice retains the selected original regex operand.
    PatternShape,
    /// Original selected template operand, independent of its runtime value.
    TemplateSubstitution,
    /// Original selected script-reparse syntax, independent of execution.
    ScriptReparse,
    /// Original channel path; source/value advice supplies no opened channel.
    ChannelPath,
    /// Original source-file path; source/value advice supplies no file entry.
    SourceFilePath,
    /// Static original value under selected credential metadata.
    CredentialLiteral,
    /// Original option values selected for channel configuration advice.
    ChannelConfiguration,
}
impl From<tcl_compiler::analyser::RegistrySourceDiagnosticKind> for RegistryPurposeDescription {
    fn from(kind: tcl_compiler::analyser::RegistrySourceDiagnosticKind) -> Self {
        use tcl_compiler::analyser::RegistrySourceDiagnosticKind as Kind;
        match kind {
            Kind::ClauseShape => Self::ClauseShape,
            Kind::ContextGate => Self::ContextGate,
            Kind::ErrorCapture => Self::ErrorCapture,
            Kind::LiteralArgument => Self::LiteralArgument,
            Kind::VariableName => Self::VariableName,
            Kind::OptionTerminator => Self::OptionTerminator,
            Kind::DisabledOption => Self::DisabledOption,
            Kind::Subcommand => Self::Subcommand,
            Kind::DeprecatedCommand => Self::DeprecatedCommand,
            Kind::Expression => Self::Expression,
            Kind::Arity => Self::Arity,
            Kind::ArgumentRelationship => Self::ArgumentRelationship,
            Kind::Lifecycle => Self::Lifecycle,
            Kind::ChannelArgument => Self::ChannelArgument,
            Kind::PackageRequirement => Self::PackageRequirement,
            Kind::PackageOrdering => Self::PackageOrdering,
            Kind::AppendList => Self::AppendList,
            Kind::CaseBody => Self::CaseBody,
            Kind::OptionOnly => Self::OptionOnly,
            Kind::PatternSubstitution => Self::PatternSubstitution,
            Kind::PatternShape => Self::PatternShape,
            Kind::IndexBounds => Self::IndexBounds,
            Kind::TemplateSubstitution => Self::TemplateSubstitution,
            Kind::ScriptReparse => Self::ScriptReparse,
            Kind::ChannelPath => Self::ChannelPath,
            Kind::SourceFilePath => Self::SourceFilePath,
            Kind::CredentialLiteral => Self::CredentialLiteral,
            Kind::ChannelConfiguration => Self::ChannelConfiguration,
        }
    }
}
impl RegistryPurposeDescription {
    fn supports(self, code: &str) -> bool {
        match self {
            Self::ClauseShape => code == "E004",
            Self::ContextGate => code == "W142",
            Self::ErrorCapture => code == "W302",
            Self::LiteralArgument => code == "W146",
            Self::VariableName => code == "W212",
            Self::OptionTerminator => code == "W304",
            Self::DisabledOption => code == "W004" || code == "W145",
            Self::Subcommand => matches!(code, "W001" | "W002" | "W145"),
            Self::DeprecatedCommand => matches!(code, "IRULE2001" | "IRULE2002"),
            Self::Expression => code == "W100",
            Self::Arity => matches!(code, "E001" | "E002" | "E003" | "E005" | "W149"),
            Self::ArgumentRelationship => matches!(code, "W147" | "W152"),
            Self::Lifecycle => matches!(code, "W135" | "W136" | "W139" | "W144" | "W150"),
            Self::ChannelArgument => code == "W126",
            Self::PackageRequirement => code == "W120",
            Self::PackageOrdering => code == "H301",
            Self::AppendList => code == "W104",
            Self::CaseBody => code == "W106",
            Self::OptionOnly => code == "W217",
            Self::PatternSubstitution => code == "W306",
            Self::PatternShape => code == "W303",
            Self::IndexBounds => matches!(code, "W230" | "W232"),
            Self::TemplateSubstitution => code == "W102",
            Self::ScriptReparse => matches!(code, "W101" | "W301" | "W309" | "W312"),
            Self::ChannelPath => code == "W103",
            Self::SourceFilePath => code == "W300",
            Self::CredentialLiteral => code == "W310",
            Self::ChannelConfiguration => code == "W311",
        }
    }
}

/// Separate source value premise; deserialising it proves no native argument.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SupplementalLiteralDescription {
    argument: usize,
    value: String,
}

/// Authored metadata key retains its independently selected Native recipe.
/// It cannot reconstruct a source package issuer or runtime package table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct NativePackageDescription {
    bytes: Vec<u8>,
    policy: PolicyDescription,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum DeclaredProvenanceDescription {
    BuiltIn,
    BundledPack,
    User,
    WorkspaceTrusted,
    WorkspaceUntrusted,
    StudioOverride,
    Document,
}
impl From<tcl_dialect::model::Provenance> for DeclaredProvenanceDescription {
    fn from(value: tcl_dialect::model::Provenance) -> Self {
        use tcl_dialect::model::Provenance;
        match value {
            Provenance::BuiltIn => Self::BuiltIn,
            Provenance::BundledPack => Self::BundledPack,
            Provenance::User => Self::User,
            Provenance::WorkspaceTrusted => Self::WorkspaceTrusted,
            Provenance::WorkspaceUntrusted => Self::WorkspaceUntrusted,
            Provenance::StudioOverride => Self::StudioOverride,
            Provenance::Document => Self::Document,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum DeclaredObligationDescription {
    DeferredLogicalBodyApplicability,
    DeclarationApplicability,
    LogicalSourceApplicability,
    UnknownEarlierMutation,
}
impl From<tcl_compiler::command_binding::OriginalDeclaredSourceObligation>
    for DeclaredObligationDescription
{
    fn from(value: tcl_compiler::command_binding::OriginalDeclaredSourceObligation) -> Self {
        use tcl_compiler::command_binding::OriginalDeclaredSourceObligation;
        match value {
            OriginalDeclaredSourceObligation::DeferredLogicalBodyApplicability => {
                Self::DeferredLogicalBodyApplicability
            }
            OriginalDeclaredSourceObligation::DeclarationApplicability => {
                Self::DeclarationApplicability
            }
            OriginalDeclaredSourceObligation::LogicalSourceApplicability => {
                Self::LogicalSourceApplicability
            }
            OriginalDeclaredSourceObligation::UnknownEarlierMutation => {
                Self::UnknownEarlierMutation
            }
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum DeclaredPurposeDescription {
    Expression,
    Arity,
    VariableName,
    ChannelArgument,
}
impl DeclaredPurposeDescription {
    fn supports(self, code: &str) -> bool {
        match self {
            Self::Expression => code == "W100",
            Self::Arity => matches!(code, "E002" | "E003" | "E005"),
            Self::VariableName => code == "W212",
            Self::ChannelArgument => code == "W126",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum CommandAvailabilityDescription {
    Unavailable,
    RuleLoaderRefused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum CommandAvailabilityDomainDescription {
    Native,
    Logical,
    Hosted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum RegisteredInstancePurposeDescription {
    MethodName,
    Arity,
    OptionRelation,
}
impl RegisteredInstancePurposeDescription {
    fn supports(self, code: &str) -> bool {
        match self {
            Self::MethodName => code == "W001",
            Self::Arity => matches!(code, "E002" | "E003"),
            Self::OptionRelation => matches!(code, "W147" | "W152"),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum RegisteredInstanceObligationDescription {
    WrittenTransitionApplicability,
    NativeBaselineSourceApplicability,
    OriginalProcedureBodyApplicability,
    FutureOriginalProcedureSourceApplicability,
    UnavailableActualLookup,
    UnknownEarlierMutation,
    ExplicitLogicalSourceApplicability,
    ConditionalDeclarationApplicability,
    DeferredLogicalBodyApplicability,
    ConditionalLogicalBodyApplicability,
    ConditionalMemberBodyCatalogueApplicability,
    ConditionalOriginalBodyCatalogueApplicability,
    ConditionalCNamespaceApplicability,
    ConditionalCBodyApplicability,
    UnretainedCSourcePathProviderApplicability,
    ConditionalCapturedLifecycleSourceApplicability,
    AuthoredCommandPrefixApplicability,
    ProducedCommandPrefixApplicability,
    RegisteredFactoryApplicability,
    RegisteredHandleBindingApplicability,
    OriginalCallbackTargetApplicability,
    UnavailableCallbackLookupFrame,
    UnacceptedSourceArity,
}
impl From<&tcl_compiler::command_binding::SourceCommandTransitionObligation>
    for RegisteredInstanceObligationDescription
{
    fn from(value: &tcl_compiler::command_binding::SourceCommandTransitionObligation) -> Self {
        use tcl_compiler::command_binding::SourceCommandTransitionObligation as Obligation;
        match value {
            Obligation::WrittenTransitionApplicability => Self::WrittenTransitionApplicability,
            Obligation::OriginalCallbackTargetApplicability => {
                Self::OriginalCallbackTargetApplicability
            }
            Obligation::UnavailableCallbackLookupFrame => Self::UnavailableCallbackLookupFrame,
            Obligation::NativeBaselineSourceApplicability => {
                Self::NativeBaselineSourceApplicability
            }
            Obligation::OriginalProcedureBodyApplicability => {
                Self::OriginalProcedureBodyApplicability
            }
            Obligation::FutureOriginalProcedureSourceApplicability => {
                Self::FutureOriginalProcedureSourceApplicability
            }
            Obligation::UnavailableActualLookup => Self::UnavailableActualLookup,
            Obligation::UnknownEarlierMutation => Self::UnknownEarlierMutation,
            Obligation::UnacceptedSourceArity => Self::UnacceptedSourceArity,
            Obligation::ExplicitLogicalSourceApplicability => {
                Self::ExplicitLogicalSourceApplicability
            }
            Obligation::ConditionalDeclarationApplicability => {
                Self::ConditionalDeclarationApplicability
            }
            Obligation::DeferredLogicalBodyApplicability => Self::DeferredLogicalBodyApplicability,
            Obligation::ConditionalLogicalBodyApplicability => {
                Self::ConditionalLogicalBodyApplicability
            }
            Obligation::ConditionalMemberBodyCatalogueApplicability => {
                Self::ConditionalMemberBodyCatalogueApplicability
            }
            Obligation::ConditionalOriginalBodyCatalogueApplicability => {
                Self::ConditionalOriginalBodyCatalogueApplicability
            }
            Obligation::ConditionalCNamespaceApplicability => {
                Self::ConditionalCNamespaceApplicability
            }
            Obligation::ConditionalCBodyApplicability => Self::ConditionalCBodyApplicability,
            Obligation::UnretainedCSourcePathProviderApplicability => {
                Self::UnretainedCSourcePathProviderApplicability
            }
            Obligation::ConditionalCapturedLifecycleSourceApplicability => {
                Self::ConditionalCapturedLifecycleSourceApplicability
            }
            Obligation::AuthoredCommandPrefixApplicability => {
                Self::AuthoredCommandPrefixApplicability
            }
            Obligation::ProducedCommandPrefixApplicability => {
                Self::ProducedCommandPrefixApplicability
            }
            Obligation::RegisteredFactoryApplicability => Self::RegisteredFactoryApplicability,
            Obligation::RegisteredHandleBindingApplicability => {
                Self::RegisteredHandleBindingApplicability
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum ObjectArityPurposeDescription {
    Constructor,
    LexicalNext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum CallbackPurposeDescription {
    Signature,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum CallbackScopeDescription {
    Unavailable,
    InvokingFrame,
    GlobalFrame,
    TriggerFrame,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum ParameterGrammarDescription {
    Tcl,
    Jim,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum CallbackCountsDescription {
    Finite { counts: Vec<usize> },
    AtLeast { minimum: usize },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum CallbackIssueDescription {
    TooFew { supplied: usize, minimum: usize },
    TooMany { supplied: usize, maximum: usize },
    NoCompatibleSignature { supplied: usize },
}
impl CallbackIssueDescription {
    fn supports(&self, code: &str) -> bool {
        match self {
            Self::TooFew { .. } => code == "E002",
            Self::TooMany { .. } => code == "E003",
            Self::NoCompatibleSignature { .. } => code == "E005",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CallbackSignatureDescription {
    simple_name_bytes: Vec<u8>,
    namespace_components: Vec<Vec<u8>>,
    policy: PolicyDescription,
    declaration_start: u32,
    parameter_grammar: ParameterGrammarDescription,
    minimum: u16,
    maximum: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum CallbackTargetKindDescription {
    LocalProcedure,
    ExternalSourceName,
    IndependentExternalHeader,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum CallbackCaptureOriginDescription {
    ResolvedHead,
    BindingPrefix { ordinal: usize },
    Written { ordinal: usize },
    ExpandedElement { written: usize, element: usize },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CallbackCaptureDescription {
    origin: CallbackCaptureOriginDescription,
    expanded: bool,
    source_literal_bytes: Option<Vec<u8>>,
    source_channel: String,
    start: u32,
    end: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CallbackSlotDescription {
    simple_name_bytes: Vec<u8>,
    namespace_components: Vec<Vec<u8>>,
    policy: PolicyDescription,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CallbackTransitionDescription {
    descriptor: String,
    start: u32,
    end: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CallbackTargetDescription {
    kind: CallbackTargetKindDescription,
    registration_start: u32,
    slot: Option<CallbackSlotDescription>,
    captured_minimum: u16,
    captured_indeterminate: bool,
    captures: Vec<CallbackCaptureDescription>,
    lineage: Vec<CallbackTransitionDescription>,
    obligations: Vec<RegisteredInstanceObligationDescription>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum PrivateTclObligationDescription {
    OriginalHeadMaterialization,
    SourcePolicyApplicability,
    UnavailableActualLookup,
    PublicRewriteEquivalence,
}
impl From<&tcl_compiler::analyser::SourcePrivateTclNamespaceObligation>
    for PrivateTclObligationDescription
{
    fn from(value: &tcl_compiler::analyser::SourcePrivateTclNamespaceObligation) -> Self {
        use tcl_compiler::analyser::SourcePrivateTclNamespaceObligation as O;
        match value {
            O::OriginalHeadMaterialization => Self::OriginalHeadMaterialization,
            O::SourcePolicyApplicability => Self::SourcePolicyApplicability,
            O::UnavailableActualLookup => Self::UnavailableActualLookup,
            O::PublicRewriteEquivalence => Self::PublicRewriteEquivalence,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum SubjectDescription {
    PrivateTclNamespace {
        #[serde(rename = "sourceName")]
        source_name: String,
        #[serde(rename = "publicCommand")]
        public_command: String,
        tail: String,
        #[serde(rename = "selectedSourceDescriptor")]
        selected_source_descriptor: bool,
        obligations: Vec<PrivateTclObligationDescription>,
        start: u32,
        end: u32,
    },
    UserSourceArity {
        #[serde(rename = "logicalSource")]
        logical_source: bool,
        #[serde(rename = "declaredMinimum")]
        declared_minimum: u16,
        #[serde(rename = "declaredMaximum")]
        declared_maximum: u16,
        #[serde(rename = "capturedArgumentCount")]
        captured_argument_count: usize,
        obligations: Vec<RegisteredInstanceObligationDescription>,
        #[serde(rename = "sourceChannel")]
        source_channel: String,
        start: u32,
        end: u32,
    },
    CallbackSourceArity {
        purpose: CallbackPurposeDescription,
        bytes: Vec<u8>,
        policy: PolicyDescription,
        scope: CallbackScopeDescription,
        #[serde(rename = "bakedArgumentCount")]
        baked_argument_count: usize,
        #[serde(rename = "appendedCounts")]
        appended_counts: CallbackCountsDescription,
        #[serde(rename = "sourceTarget")]
        source_target: Option<CallbackTargetDescription>,
        counts: CallbackCountsDescription,
        issue: CallbackIssueDescription,
        declarations: Vec<CallbackSignatureDescription>,
        #[serde(rename = "sourceChannel")]
        source_channel: String,
        start: u32,
        end: u32,
    },
    ObjectSourceArity {
        purpose: ObjectArityPurposeDescription,
        obligations: Vec<RegisteredInstanceObligationDescription>,
        #[serde(rename = "classSimpleNameBytes")]
        class_simple_name_bytes: Vec<u8>,
        #[serde(rename = "classNamespaceComponents")]
        class_namespace_components: Vec<Vec<u8>>,
        policy: PolicyDescription,
        #[serde(rename = "declarationStart")]
        declaration_start: u32,
        #[serde(rename = "sourceChannel")]
        source_channel: String,
        #[serde(rename = "minimumArgumentCount")]
        minimum_argument_count: u16,
        #[serde(rename = "indeterminateArgumentCount")]
        indeterminate_argument_count: bool,
        start: u32,
        end: u32,
    },
    CommandAvailability {
        command: String,
        classification: CommandAvailabilityDescription,
        domain: CommandAvailabilityDomainDescription,
        #[serde(rename = "sourceChannel")]
        source_channel: String,
        start: u32,
        end: u32,
    },
    RegisteredInstanceSource {
        purpose: RegisteredInstancePurposeDescription,
        #[serde(rename = "factoryCommand")]
        factory_command: String,
        #[serde(rename = "instanceNameBytes")]
        instance_name_bytes: Vec<u8>,
        #[serde(rename = "factoryStart")]
        factory_start: u32,
        #[serde(rename = "sourceChannel")]
        source_channel: String,
        obligations: Vec<RegisteredInstanceObligationDescription>,
        start: u32,
        end: u32,
    },
    DeclaredSource {
        purpose: DeclaredPurposeDescription,
        #[serde(rename = "declarationName")]
        declaration_name: String,
        provenance: DeclaredProvenanceDescription,
        obligations: Vec<DeclaredObligationDescription>,
        argument: Option<usize>,
        #[serde(rename = "sourceChannel")]
        source_channel: String,
        start: u32,
        end: u32,
    },
    RegistrySource {
        purpose: RegistryPurposeDescription,
        #[serde(
            rename = "supplementalLiterals",
            default,
            skip_serializing_if = "Vec::is_empty"
        )]
        supplemental_literals: Vec<SupplementalLiteralDescription>,
        #[serde(
            rename = "listElementPath",
            default,
            skip_serializing_if = "Vec::is_empty"
        )]
        list_element_path: Vec<usize>,
        command: String,
        argument: Option<usize>,
        #[serde(rename = "writtenArgument")]
        written_argument: Option<usize>,
        #[serde(rename = "sourceChannel")]
        source_channel: String,
        start: u32,
        end: u32,
        #[serde(
            rename = "requiredPackage",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        required_package: Option<String>,
        #[serde(
            rename = "nativePackage",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        native_package: Option<NativePackageDescription>,
    },
    ConditionalInterpreterVisibility {
        bytes: Vec<u8>,
        policy: PolicyDescription,
        #[serde(rename = "sourceChannel")]
        source_channel: String,
        start: u32,
        end: u32,
        #[serde(rename = "rootSlot")]
        root_slot: Vec<u8>,
        obligations: Vec<VisibilityObligationDescription>,
    },
    RequiredPackage {
        bytes: Vec<u8>,
        policy: PolicyDescription,
    },
    UnresolvedMathFunction {
        bytes: Vec<u8>,
        policy: PolicyDescription,
        #[serde(rename = "reportingName")]
        reporting_name: String,
        #[serde(rename = "sourceChannel")]
        source_channel: String,
        start: u32,
        end: u32,
        ordinal: usize,
        #[serde(rename = "argumentCount")]
        argument_count: usize,
        dispatch: MathDispatchDescription,
        classification: MathClassificationDescription,
    },
    UnresolvedCommand {
        bytes: Vec<u8>,
        policy: PolicyDescription,
        #[serde(rename = "reportingName")]
        reporting_name: String,
        #[serde(rename = "sourceChannel")]
        source_channel: String,
        start: u32,
        end: u32,
    },
}

/// Reporting data retained independently of a diagnostic's presentation.
///
/// Native names remain counted bytes. This data supplies no resolution,
/// allocation, execution, original-source ownership or writable-reference proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiagnosticSubjectData {
    version: u8,
    code: String,
    subject: SubjectDescription,
}

fn command_availability_description(
    original: &tcl_compiler::registry_invocation::OriginalSourceCommandAvailability,
) -> SubjectDescription {
    use tcl_compiler::registry_invocation::CommandSourceAvailabilityDomain as Domain;
    use tcl_registry::model::CommandSourceUnavailabilityKind as Kind;
    SubjectDescription::CommandAvailability {
        command: original.descriptor().command().to_owned(),
        classification: match original.descriptor().kind() {
            Kind::Unavailable => CommandAvailabilityDescription::Unavailable,
            Kind::RuleLoaderRefused => CommandAvailabilityDescription::RuleLoaderRefused,
        },
        domain: match original.domain() {
            Domain::Native => CommandAvailabilityDomainDescription::Native,
            Domain::Logical => CommandAvailabilityDomainDescription::Logical,
            Domain::Hosted => CommandAvailabilityDomainDescription::Hosted,
        },
        source_channel: match original.original_head().image().channel() {
            tcl_lexer::SourceChannel::Document => "document",
            tcl_lexer::SourceChannel::NativeValue => "nativeValue",
        }
        .to_owned(),
        start: original.original_head().span().start(),
        end: original.original_head().span().end(),
    }
}

fn callback_capture_description(
    target: &tcl_compiler::command_binding::OriginalSourceCallbackProcedureTarget,
    ordinal: usize,
) -> Option<CallbackCaptureDescription> {
    use tcl_compiler::registry_invocation::InvocationWordOrigin as Origin;
    let word = target.argument_word(ordinal)?;
    Some(CallbackCaptureDescription {
        origin: match target.argument_origin(ordinal)? {
            Origin::ResolvedHead => CallbackCaptureOriginDescription::ResolvedHead,
            Origin::BindingPrefix(ordinal) => {
                CallbackCaptureOriginDescription::BindingPrefix { ordinal: *ordinal }
            }
            Origin::Written(ordinal) => {
                CallbackCaptureOriginDescription::Written { ordinal: *ordinal }
            }
            Origin::ExpandedElement { written, element } => {
                CallbackCaptureOriginDescription::ExpandedElement {
                    written: *written,
                    element: *element,
                }
            }
        },
        expanded: word.group().expand,
        source_literal_bytes: target
            .argument_input(ordinal)
            .map(|input| input.bytes().to_vec()),
        source_channel: match word.image().channel() {
            tcl_lexer::SourceChannel::Document => "document",
            tcl_lexer::SourceChannel::NativeValue => "nativeValue",
        }
        .to_owned(),
        start: word.span().start(),
        end: word.span().end(),
    })
}
fn callback_target_description(
    selection: &tcl_compiler::analyser::SourceCallbackSignatureLookup,
) -> Option<CallbackTargetDescription> {
    use tcl_compiler::command_binding::OriginalSourceCallbackProcedureTargetKind as Kind;
    let original = selection.original();
    let count = selection.captured_argument_count()?;
    let (kind, slot, captures, lineage) = if let Some(target) = original.target() {
        let slot = target.source_slot();
        let policy = target.target_input().native_input()?.policy();
        (
            match target.kind() {
                Kind::LocalProcedure => CallbackTargetKindDescription::LocalProcedure,
                Kind::ExternalSourceName => CallbackTargetKindDescription::ExternalSourceName,
            },
            Some(CallbackSlotDescription {
                simple_name_bytes: slot.simple.as_bytes().to_vec(),
                namespace_components: slot
                    .namespace
                    .as_segments()
                    .iter()
                    .map(|part| part.as_bytes().to_vec())
                    .collect(),
                policy: policy.into(),
            }),
            (0..target.captured_arguments().len())
                .map(|ordinal| callback_capture_description(target, ordinal))
                .collect::<Option<Vec<_>>>()?,
            target
                .lineage()
                .iter()
                .map(|transition| {
                    Some(CallbackTransitionDescription {
                        descriptor: transition.descriptor().to_owned(),
                        start: transition.original_words().first()?.span().start(),
                        end: transition.original_words().last()?.span().end(),
                    })
                })
                .collect::<Option<Vec<_>>>()?,
        )
    } else if original.permits_external_signature_lookup() {
        (
            CallbackTargetKindDescription::IndependentExternalHeader,
            None,
            Vec::new(),
            Vec::new(),
        )
    } else {
        return None;
    };
    Some(CallbackTargetDescription {
        kind,
        registration_start: original.registration().site().offset,
        slot,
        captured_minimum: count.minimum,
        captured_indeterminate: count.indeterminate,
        captures,
        lineage,
        obligations: original
            .obligations()
            .iter()
            .map(RegisteredInstanceObligationDescription::from)
            .collect(),
    })
}

fn callback_arity_description(
    original: &tcl_compiler::analyser::SourceCallbackAritySubject,
) -> Option<SubjectDescription> {
    use tcl_compiler::analyser::{
        SourceCallbackArgumentCounts as Counts, SourceCallbackArityIssue as Issue,
    };
    let prefix = original.prefix();
    let declarations = original
        .declarations()
        .iter()
        .map(|header| {
            let projection = header.formal_count_projection();
            let arity = projection.arity();
            Some(CallbackSignatureDescription {
                simple_name_bytes: header.name().slot().simple.as_bytes().to_vec(),
                namespace_components: header
                    .name()
                    .slot()
                    .namespace
                    .as_segments()
                    .iter()
                    .map(|component| component.as_bytes().to_vec())
                    .collect(),
                policy: header.name().policy().into(),
                declaration_start: header.declaration_offset(),
                parameter_grammar: match projection.parameter_grammar()? {
                    tcl_dialect::ParameterGrammar::Tcl => ParameterGrammarDescription::Tcl,
                    tcl_dialect::ParameterGrammar::Jim => ParameterGrammarDescription::Jim,
                },
                minimum: arity.min,
                maximum: (!arity.is_unlimited()).then_some(arity.max),
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(SubjectDescription::CallbackSourceArity {
        purpose: CallbackPurposeDescription::Signature,
        bytes: prefix.name_input().bytes().to_vec(),
        policy: prefix.name_input().policy().into(),
        scope: match prefix.scope() {
            None => CallbackScopeDescription::Unavailable,
            Some(tcl_registry::ScriptLookupScope::InvokingFrame) => {
                CallbackScopeDescription::InvokingFrame
            }
            Some(tcl_registry::ScriptLookupScope::GlobalFrame) => {
                CallbackScopeDescription::GlobalFrame
            }
            Some(tcl_registry::ScriptLookupScope::TriggerFrame) => {
                CallbackScopeDescription::TriggerFrame
            }
        },
        baked_argument_count: prefix.baked_argument_count(),
        appended_counts: {
            let appended = prefix.appended_arity()?;
            if let Some(exact) = appended.exact_counts() {
                CallbackCountsDescription::Finite {
                    counts: exact.map(usize::from).collect(),
                }
            } else {
                CallbackCountsDescription::AtLeast {
                    minimum: usize::from(appended.min()),
                }
            }
        },
        source_target: if let Some(selection) = original.source_lookup() {
            Some(callback_target_description(selection)?)
        } else {
            None
        },
        counts: match original.argument_counts() {
            Counts::Finite(counts) => CallbackCountsDescription::Finite {
                counts: counts.clone(),
            },
            Counts::AtLeast(minimum) => CallbackCountsDescription::AtLeast { minimum: *minimum },
        },
        issue: match original.issue() {
            Issue::TooFew {
                supplied,
                expected_minimum,
            } => CallbackIssueDescription::TooFew {
                supplied,
                minimum: expected_minimum,
            },
            Issue::TooMany {
                supplied,
                expected_maximum,
            } => CallbackIssueDescription::TooMany {
                supplied,
                maximum: expected_maximum,
            },
            Issue::NoCompatibleSignature { supplied } => {
                CallbackIssueDescription::NoCompatibleSignature { supplied }
            }
        },
        declarations,
        source_channel: match original.source_channel() {
            tcl_lexer::SourceChannel::Document => "document",
            tcl_lexer::SourceChannel::NativeValue => "nativeValue",
        }
        .to_owned(),
        start: original.span().start(),
        end: original.span().end(),
    })
}

fn object_arity_description(
    original: &tcl_compiler::analyser::ObjectSourceAritySubject,
) -> Option<SubjectDescription> {
    match original {
        tcl_compiler::analyser::ObjectSourceAritySubject::Constructor(advice) => {
            let call = advice.call().original_call();
            let class = call.class_declaration().source_slot();
            let first = call.original_words().first()?;
            let count = advice.argument_count();
            Some(SubjectDescription::ObjectSourceArity {
                purpose: ObjectArityPurposeDescription::Constructor,
                obligations: call
                    .obligations()
                    .iter()
                    .map(RegisteredInstanceObligationDescription::from)
                    .collect(),
                class_simple_name_bytes: class.simple.as_bytes().to_vec(),
                class_namespace_components: class
                    .namespace
                    .as_segments()
                    .iter()
                    .map(|part| part.as_bytes().to_vec())
                    .collect(),
                policy: call
                    .class_declaration()
                    .name_input()
                    .native_input()?
                    .policy()
                    .into(),
                declaration_start: call.class_declaration().factory().site().offset,
                source_channel: match first.image().channel() {
                    tcl_lexer::SourceChannel::Document => "document",
                    tcl_lexer::SourceChannel::NativeValue => "nativeValue",
                }
                .to_owned(),
                minimum_argument_count: count.minimum,
                indeterminate_argument_count: count.indeterminate,
                start: first.span().start(),
                end: call.original_words().last()?.span().end(),
            })
        }
        tcl_compiler::analyser::ObjectSourceAritySubject::LexicalNext(next) => {
            let context = next.member_context();
            let class = context.class_publication();
            let first = next.original_words().first()?;
            let count = next.source_argument_count()?;
            Some(SubjectDescription::ObjectSourceArity {
                purpose: ObjectArityPurposeDescription::LexicalNext,
                obligations: Vec::new(),
                class_simple_name_bytes: class.slot().simple.as_bytes().to_vec(),
                class_namespace_components: class
                    .slot()
                    .namespace
                    .as_segments()
                    .iter()
                    .map(|component| component.as_bytes().to_vec())
                    .collect(),
                policy: class.policy().into(),
                declaration_start: context.declaration_site().offset,
                source_channel: match first.image().channel() {
                    tcl_lexer::SourceChannel::Document => "document",
                    tcl_lexer::SourceChannel::NativeValue => "nativeValue",
                }
                .to_owned(),
                minimum_argument_count: count.minimum,
                indeterminate_argument_count: count.indeterminate,
                start: first.span().start(),
                end: next.original_words().last()?.span().end(),
            })
        }
    }
}

fn arity_subject_description(
    diagnostic: &Diagnostic,
    subject: &DiagnosticSubject,
) -> Option<SubjectDescription> {
    Some(match subject {
        DiagnosticSubject::PrivateTclNamespace(original) => {
            if diagnostic.code != tcl_core_types::DiagCode::W143 {
                return None;
            }
            SubjectDescription::PrivateTclNamespace {
                source_name: original.classified_source_name().to_owned(),
                public_command: original.policy().public_command.to_owned(),
                tail: original.policy().tail.clone(),
                selected_source_descriptor: original.selected_source_words().is_some(),
                obligations: original.obligations().iter().map(Into::into).collect(),
                start: original.original_head().span().start(),
                end: original.original_head().span().end(),
            }
        }
        DiagnosticSubject::UserSourceArity(original) => {
            if !matches!(
                diagnostic.code,
                tcl_core_types::DiagCode::E002 | tcl_core_types::DiagCode::E003
            ) {
                return None;
            }
            let words = original.original_words();
            let first = words.first()?;
            let arity = original.declared_arity();
            SubjectDescription::UserSourceArity {
                logical_source: original.is_logical_source(),
                declared_minimum: arity.min,
                declared_maximum: arity.max,
                captured_argument_count: original.captured_argument_count(),
                obligations: original.obligations().iter().map(Into::into).collect(),
                source_channel: match first.image().channel() {
                    tcl_lexer::SourceChannel::Document => "document",
                    tcl_lexer::SourceChannel::NativeValue => "nativeValue",
                }
                .to_owned(),
                start: first.span().start(),
                end: words.last()?.span().end(),
            }
        }
        DiagnosticSubject::CallbackSourceArity(original) => {
            if original.issue().code() != diagnostic.code {
                return None;
            }
            callback_arity_description(original)?
        }
        DiagnosticSubject::ObjectSourceArity(original) => object_arity_description(original)?,
        DiagnosticSubject::CommandAvailability(original) => {
            command_availability_description(original)
        }
        _ => return None,
    })
}

fn declared_subject_description(subject: &DiagnosticSubject) -> Option<SubjectDescription> {
    Some(match subject {
        DiagnosticSubject::RegisteredInstanceSource(original) => {
            let words = original.words();
            let word = words.original_words().first()?;
            SubjectDescription::RegisteredInstanceSource {
                purpose: match original.kind() {
                    tcl_compiler::analyser::RegisteredInstanceSourceDiagnosticKind::MethodName => RegisteredInstancePurposeDescription::MethodName,
                    tcl_compiler::analyser::RegisteredInstanceSourceDiagnosticKind::Arity => RegisteredInstancePurposeDescription::Arity,
                    tcl_compiler::analyser::RegisteredInstanceSourceDiagnosticKind::OptionRelation => RegisteredInstancePurposeDescription::OptionRelation,
                },
                factory_command: words.instance().factory().command().to_owned(),
                instance_name_bytes: words.instance().name_input().bytes().to_vec(),
                factory_start: words.instance().factory().site().offset,
                source_channel: match word.image().channel() {
                    tcl_lexer::SourceChannel::Document => "document",
                    tcl_lexer::SourceChannel::NativeValue => "nativeValue",
                }.to_owned(),
                obligations: words.obligations().iter().map(Into::into).collect(),
                start: word.span().start(),
                end: words.original_words().last()?.span().end(),
            }
        }
        DiagnosticSubject::DeclaredSource(original) => {
            let word = original.words().original_words().first()?;
            SubjectDescription::DeclaredSource {
                purpose: match original.kind() {
                    tcl_compiler::analyser::DeclaredSourceDiagnosticKind::Expression => {
                        DeclaredPurposeDescription::Expression
                    }
                    tcl_compiler::analyser::DeclaredSourceDiagnosticKind::Arity => {
                        DeclaredPurposeDescription::Arity
                    }
                    tcl_compiler::analyser::DeclaredSourceDiagnosticKind::VariableName => {
                        DeclaredPurposeDescription::VariableName
                    }
                    tcl_compiler::analyser::DeclaredSourceDiagnosticKind::ChannelArgument => {
                        DeclaredPurposeDescription::ChannelArgument
                    }
                },
                declaration_name: original.words().descriptor().name.clone(),
                provenance: original.words().descriptor().provenance().into(),
                obligations: original
                    .words()
                    .obligations()
                    .iter()
                    .copied()
                    .map(Into::into)
                    .collect(),
                argument: original.argument(),
                source_channel: match word.image().channel() {
                    tcl_lexer::SourceChannel::Document => "document",
                    tcl_lexer::SourceChannel::NativeValue => "nativeValue",
                }
                .to_owned(),
                start: original.span().start(),
                end: original.span().end(),
            }
        }
        _ => return None,
    })
}

fn registry_subject_description(subject: &DiagnosticSubject) -> Option<SubjectDescription> {
    Some(match subject {
        DiagnosticSubject::RegistrySource(original) => {
            let word = original.words().head_source()?.word()?;
            SubjectDescription::RegistrySource {
                purpose: original.kind().into(),
                supplemental_literals: original
                    .supplemental_literals()
                    .iter()
                    .map(|(argument, value)| SupplementalLiteralDescription {
                        argument: *argument,
                        value: value.clone(),
                    })
                    .collect(),
                list_element_path: original.list_element_path().to_vec(),
                command: original.words().command().to_owned(),
                argument: original.argument(),
                written_argument: original.written_argument(),
                source_channel: match word.image().channel() {
                    tcl_lexer::SourceChannel::Document => "document",
                    tcl_lexer::SourceChannel::NativeValue => "nativeValue",
                }
                .to_owned(),
                start: original.span().start(),
                end: original.span().end(),
                required_package: original.required_package().map(str::to_owned),
                native_package: original.required_package_key().map(|key| {
                    NativePackageDescription {
                        bytes: key.bytes().to_vec(),
                        policy: key.policy().into(),
                    }
                }),
            }
        }
        DiagnosticSubject::ConditionalInterpreterVisibility(original) => {
            let key = original.name_input().original_word_key()?;
            SubjectDescription::ConditionalInterpreterVisibility {
                bytes: original.name_input().bytes().to_vec(),
                policy: original.name_input().policy().into(),
                source_channel: match key.source_image().channel() {
                    tcl_lexer::SourceChannel::Document => "document",
                    tcl_lexer::SourceChannel::NativeValue => "nativeValue",
                }
                .to_owned(),
                start: key.span().start(),
                end: key.span().end(),
                root_slot: original.slot().simple.as_bytes().to_vec(),
                obligations: original
                    .obligations()
                    .iter()
                    .copied()
                    .map(Into::into)
                    .collect(),
            }
        }
        DiagnosticSubject::RequiredPackage(key) => SubjectDescription::RequiredPackage {
            bytes: key.bytes().to_vec(),
            policy: key.policy().into(),
        },
        _ => return None,
    })
}

fn unresolved_subject_description(subject: &DiagnosticSubject) -> Option<SubjectDescription> {
    Some(match subject {
        DiagnosticSubject::UnresolvedMathFunction(original) => {
            let occurrence = original.occurrence();
            SubjectDescription::UnresolvedMathFunction {
                bytes: occurrence.bytes().to_vec(),
                policy: occurrence.name_policy().into(),
                reporting_name: original.reporting_name().to_owned(),
                source_channel: match occurrence.source_image().channel() {
                    tcl_lexer::SourceChannel::NativeValue => "nativeValue",
                    tcl_lexer::SourceChannel::Document => "document",
                }
                .to_owned(),
                start: occurrence.span().start(),
                end: occurrence.span().end(),
                ordinal: occurrence.ordinal(),
                argument_count: occurrence.argument_count(),
                dispatch: match occurrence.dispatch() {
                    tcl_registry::mathfunc::NativeMathFunctionDispatch::FixedTable => {
                        MathDispatchDescription::FixedTable
                    }
                    tcl_registry::mathfunc::NativeMathFunctionDispatch::CommandTable => {
                        MathDispatchDescription::CommandTable
                    }
                },
                classification: MathClassificationDescription::Absent,
            }
        }
        DiagnosticSubject::UnresolvedCommand(original) => {
            let key = original.name_input();
            SubjectDescription::UnresolvedCommand {
                bytes: key.bytes().to_vec(),
                policy: key.policy().into(),
                reporting_name: original.reporting_name().to_owned(),
                source_channel: match key.source_image().channel() {
                    tcl_lexer::SourceChannel::NativeValue => "nativeValue",
                    tcl_lexer::SourceChannel::Document => "document",
                }
                .to_owned(),
                start: key.span().start(),
                end: key.span().end(),
            }
        }
        _ => return None,
    })
}

impl DiagnosticSubjectData {
    /// Project the emitting owner's typed subject without parsing presentation.
    #[must_use]
    pub fn from_diagnostic(diagnostic: &Diagnostic) -> Option<Self> {
        let original = diagnostic.subject()?;
        let subject = match original {
            DiagnosticSubject::PrivateTclNamespace(_)
            | DiagnosticSubject::UserSourceArity(_)
            | DiagnosticSubject::CallbackSourceArity(_)
            | DiagnosticSubject::ObjectSourceArity(_)
            | DiagnosticSubject::CommandAvailability(_) => {
                arity_subject_description(diagnostic, original)?
            }
            DiagnosticSubject::RegisteredInstanceSource(_)
            | DiagnosticSubject::DeclaredSource(_) => declared_subject_description(original)?,
            DiagnosticSubject::RegistrySource(_)
            | DiagnosticSubject::ConditionalInterpreterVisibility(_)
            | DiagnosticSubject::RequiredPackage(_) => registry_subject_description(original)?,
            DiagnosticSubject::UnresolvedMathFunction(_)
            | DiagnosticSubject::UnresolvedCommand(_) => unresolved_subject_description(original)?,
        };
        Some(Self {
            version: 1,
            code: diagnostic.code.to_string(),
            subject,
        })
    }

    /// Decode supported reporting data for the expected diagnostic code.
    /// Missing, malformed and future payloads stay unknown; no message fallback.
    #[must_use]
    pub fn from_value(value: &Value, code: &str) -> Option<Self> {
        let data: Self = serde_json::from_value(value.clone()).ok()?;
        let supported_subject = matches!(
            (&data.subject, data.code.as_str()),
            (SubjectDescription::PrivateTclNamespace { .. }, "W143")
                | (
                    SubjectDescription::ObjectSourceArity { .. }
                        | SubjectDescription::UserSourceArity { .. },
                    "E002" | "E003"
                )
                | (
                    SubjectDescription::CommandAvailability {
                        classification: CommandAvailabilityDescription::Unavailable,
                        ..
                    },
                    "W002"
                )
                | (
                    SubjectDescription::CommandAvailability {
                        classification: CommandAvailabilityDescription::RuleLoaderRefused,
                        ..
                    },
                    "IRULE2004"
                )
                | (SubjectDescription::RequiredPackage { .. }, "W120")
                | (
                    SubjectDescription::UnresolvedCommand { .. }
                        | SubjectDescription::UnresolvedMathFunction { .. },
                    "W123"
                )
                | (
                    SubjectDescription::ConditionalInterpreterVisibility { .. },
                    "W129"
                )
        );
        let supported_subject = supported_subject
            || matches!(&data.subject, SubjectDescription::CallbackSourceArity { issue, .. } if issue.supports(&data.code))
            || matches!(&data.subject,
            SubjectDescription::RegistrySource { purpose, .. } if purpose.supports(&data.code))
            || matches!(&data.subject, SubjectDescription::DeclaredSource { purpose, .. } if purpose.supports(&data.code))
            || matches!(&data.subject, SubjectDescription::RegisteredInstanceSource { purpose, .. } if purpose.supports(&data.code));
        (data.version == 1 && data.code == code && supported_subject).then_some(data)
    }

    /// Encode the shared transport used by every protocol adapter.
    #[must_use]
    pub fn to_value(&self) -> Option<Value> {
        serde_json::to_value(self).ok()
    }

    /// Conditional child-source subject bytes; the payload cannot reconstruct
    /// a child interpreter, original invocation or selected hidden allocation.
    #[must_use]
    pub fn conditional_visibility_bytes(&self) -> Option<&[u8]> {
        match &self.subject {
            SubjectDescription::ConditionalInterpreterVisibility { bytes, .. } => Some(bytes),
            _ => None,
        }
    }

    /// Exact reporting package bytes, without reconstructing a naming issuer.
    #[must_use]
    pub fn package_bytes(&self) -> Option<&[u8]> {
        match &self.subject {
            SubjectDescription::RequiredPackage { bytes, .. } => Some(bytes),
            SubjectDescription::RegistrySource { native_package, .. } => {
                native_package.as_ref().map(|key| key.bytes.as_slice())
            }
            SubjectDescription::CallbackSourceArity { .. }
            | SubjectDescription::ObjectSourceArity { .. }
            | SubjectDescription::PrivateTclNamespace { .. }
            | SubjectDescription::UserSourceArity { .. }
            | SubjectDescription::CommandAvailability { .. }
            | SubjectDescription::RegisteredInstanceSource { .. }
            | SubjectDescription::DeclaredSource { .. }
            | SubjectDescription::UnresolvedCommand { .. }
            | SubjectDescription::UnresolvedMathFunction { .. }
            | SubjectDescription::ConditionalInterpreterVisibility { .. } => None,
        }
    }

    /// Original emitting word's reporting label; it supplies no lookup proof.
    #[must_use]
    pub fn command_reporting_name(&self) -> Option<&str> {
        match &self.subject {
            SubjectDescription::UnresolvedCommand { reporting_name, .. } => Some(reporting_name),
            SubjectDescription::CallbackSourceArity { .. }
            | SubjectDescription::ObjectSourceArity { .. }
            | SubjectDescription::PrivateTclNamespace { .. }
            | SubjectDescription::UserSourceArity { .. }
            | SubjectDescription::CommandAvailability { .. }
            | SubjectDescription::RegisteredInstanceSource { .. }
            | SubjectDescription::DeclaredSource { .. }
            | SubjectDescription::RegistrySource { .. }
            | SubjectDescription::RequiredPackage { .. }
            | SubjectDescription::UnresolvedMathFunction { .. }
            | SubjectDescription::ConditionalInterpreterVisibility { .. } => None,
        }
    }
    /// Original expression identifier reporting; no command word is recovered.
    #[must_use]
    pub fn math_function_reporting_name(&self) -> Option<&str> {
        match &self.subject {
            SubjectDescription::UnresolvedMathFunction { reporting_name, .. } => {
                Some(reporting_name)
            }
            SubjectDescription::CallbackSourceArity { .. }
            | SubjectDescription::ObjectSourceArity { .. }
            | SubjectDescription::PrivateTclNamespace { .. }
            | SubjectDescription::UserSourceArity { .. }
            | SubjectDescription::CommandAvailability { .. }
            | SubjectDescription::RegisteredInstanceSource { .. }
            | SubjectDescription::DeclaredSource { .. }
            | SubjectDescription::RegistrySource { .. }
            | SubjectDescription::RequiredPackage { .. }
            | SubjectDescription::UnresolvedCommand { .. }
            | SubjectDescription::ConditionalInterpreterVisibility { .. } => None,
        }
    }
}

/// Shared serialized subject projection; absent owner data remains absent.
#[must_use]
pub fn diagnostic_subject_data(diagnostic: &Diagnostic) -> Option<Value> {
    DiagnosticSubjectData::from_diagnostic(diagnostic)?.to_value()
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod conditional_user_signature_transport_controls;

#[cfg(test)]
mod private_tcl_source_policy_wire_controls;
